use libloading as lib;
use tempfile;
use fxhash::FxHashMap as HashMap;
#[cfg(unix)]
use libloading::os::unix as libimp;
#[cfg(windows)]
use libloading::os::windows as libimp;
use notify::{self, Watcher};

use system::{System, SystemThreadLocal, SystemWithData, SystemWithDataThreadLocal, CreationSystem, CreationSystemWithData};
use ::Entities;
use ::Resources;
use ::EntitiesThreadLocal;
use ::ResourcesThreadLocal;
use ::EntitiesCreation;

use std::process::Command;
use std::error::Error;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, PoisonError};
use std::collections::hash_map::Entry;
use std::cell::UnsafeCell;
use std::thread;
use std::env::current_dir;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::ffi::CString;
use std::time::Duration;
use std::mem;
use std::os::raw::c_void;


#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct SystemPath{
    library: String,
    system: String,
}

impl SystemPath{
    fn new(system_path: &str) -> Result<SystemPath, String>{
        let mut system_parts = system_path.split("::");
        let lib_name = system_parts.next().expect("Empty system path");
        let system_name = system_parts.next()
            .expect(&format!("Can't find system in path with onlyread().unwrap(). one part {}, system paths have to be specified as library::system", system_path));
        if system_parts.next().is_none() {
            Ok(SystemPath{
                library: lib_name.to_owned(),
                system: system_name.to_owned(),
            })
        }else{
            Err("System path has more than two elements, correct format is: library::system".to_string())
        }
    }
}

struct Data{
    libraries: HashMap<PathBuf, DynamicLibrary>,
    library_names_index: HashMap<PathBuf, String>, //TODO: this can probably be a vector or use src path as index instead
    systems: DynamicSystemLoader<fn(Entities, Resources)>,
    systems_thread_local: DynamicSystemLoader<fn(EntitiesThreadLocal, ResourcesThreadLocal)>,
    systems_with_data: DynamicSystemLoader<fn(*mut c_void, Entities, Resources)>,
    systems_with_data_thread_local: DynamicSystemLoader<fn(*mut c_void, EntitiesThreadLocal, ResourcesThreadLocal)>,
    creation_systems: DynamicSystemLoader<fn(EntitiesCreation, ResourcesThreadLocal)>,
    creation_systems_with_data: DynamicSystemLoader<fn(*mut c_void, EntitiesCreation, ResourcesThreadLocal)>,
    source_rx: Receiver<notify::DebouncedEvent>,
    source_watcher: notify::RecommendedWatcher,
    libs_rx: Receiver<notify::DebouncedEvent>,
    libs_watcher: notify::RecommendedWatcher,
    done: bool,
}

pub struct DynamicSystemsLoader{
    data: Arc<Mutex<Data>>,
    // updater: thread::JoinHandle,
}

impl DynamicSystemsLoader{
    pub fn new() -> Result<DynamicSystemsLoader,String>{
        let data = Arc::new(Mutex::new(Data::new()?));
        let loader_data = data.clone();
        let updater = thread::spawn(move ||{
            DynamicSystemsLoader::update(data);
        });
        Ok(DynamicSystemsLoader{
            data: loader_data,
            // updater,
        })
    }

    pub fn new_system(&mut self, system_path: &str) -> Result<DynamicSystem, String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .new_system(system_path)
    }

    pub fn new_system_thread_local(&mut self, system_path: &str) -> Result<DynamicSystemThreadLocal, String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .new_system_thread_local(system_path)
    }

    pub fn new_system_with_data(&mut self, system_path: &str) -> Result<DynamicSystemWithData, String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .new_system_with_data(system_path)
    }

    pub fn new_system_with_data_thread_local(&mut self, system_path: &str) -> Result<DynamicSystemWithDataThreadLocal, String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .new_system_with_data_thread_local(system_path)
    }

    pub fn new_creation_system(&mut self, system_path: &str) -> Result<DynamicCreationSystem, String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .new_creation_system(system_path)
    }

    pub fn new_creation_system_with_data(&mut self, system_path: &str) -> Result<DynamicCreationSystemWithData, String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .new_creation_system_with_data(system_path)
    }

    pub fn start(&mut self) -> Result<(), String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .start()
    }

    pub fn preload_libraries(&mut self, libs: &[&str]) -> Result<(), String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .preload_libraries(libs)
    }

    fn recompile(libraries: &[&str]) {
        println!("Recompiling {:?}", libraries);

        let mut args = vec!["build"];
        #[cfg(not(debug_assertions))]
        args.push("--release");

        for library in libraries {
            args.push("-p");
            args.push(library);
        }

        match Command::new("cargo")
            .args(&args)
            .output()
        {
            Ok(output) => {
                let stderr = CString::new(output.stderr).unwrap().into_string().unwrap();
                let stdout = CString::new(output.stdout).unwrap().into_string().unwrap();
                if output.status.success() {
                    println!("Built succesfully {}\n{}", stdout, stderr);
                }else{
                    println!("Error building system {}\n{}", stdout, stderr);
                }
            }
            Err(err) => println!("Error building system {}", err.description()),
        }
    }

    fn update(data: Arc<Mutex<Data>>){
        loop {
            let mut data = data.lock().unwrap();
            if data.done {
                return;
            }

            data.update_source();
            data.update_libs();

        }
    }
}

fn temporary_library<P: AsRef<Path>>(lib_path: P) -> Result<(lib::Library, tempfile::TempPath), String>{
    let mut templib = tempfile::NamedTempFile::new()
        .map_err(|e| format!("Couldn't create temporary library: {}", e.description()))?;
    let mut originallib = File::open(lib_path)
        .map_err(|e| format!("Couldn't open library: {}", e.description()))?;
    let mut buf = vec![];
    originallib.read_to_end(&mut buf)
        .map_err(|e| format!("Couldn't read library for temporary copy: {}", e.description()))?;
    templib.write_all(&buf)
        .map_err(|e| format!("Couldn't write temporary library copy: {}", e.description()))?;

    lib::Library::new(templib.path())
        .map_err(|e| e.description().to_owned())
        .map(|l| (l, templib.into_temp_path()))
}

impl Data{
    fn new() -> Result<Data, String>{
        let (tx, libs_rx) = channel();
        let libs_watcher: notify::RecommendedWatcher =
            notify::Watcher::new(tx, Duration::from_secs(1))
                .map_err(|e| format!("Error creating watcher: {}", e.description()))?;
        let (tx, source_rx) = channel();
        let source_watcher = notify::Watcher::new(tx, Duration::from_secs(1))
            .map_err(|e| format!("Error creating watcher: {}", e.description()))?;

        Ok(Data{
            libraries: HashMap::default(),
            systems: DynamicSystemLoader::new(),
            systems_thread_local: DynamicSystemLoader::new(),
            systems_with_data: DynamicSystemLoader::new(),
            systems_with_data_thread_local: DynamicSystemLoader::new(),
            creation_systems: DynamicSystemLoader::new(),
            creation_systems_with_data: DynamicSystemLoader::new(),
            library_names_index: HashMap::default(),
            source_watcher,
            libs_watcher,
            source_rx,
            libs_rx,
            done: false,
        })
    }

    fn start(&mut self) -> Result<(), String>{
        // let mut lib_path = PathBuf::from("target");constructor_name

        // #[cfg(debug_assertions)]
        // lib_path.push("debug");

        // #[cfg(not(debug_assertions))]
        // lib_path.push("release");

        let exe_path = ::std::env::current_exe()
            .map_err(|e| format!("Error trying to figure out dynamic library folder: {}", e.description()))?;
        let lib_path = exe_path.parent().unwrap();
        self.libs_watcher.watch(&lib_path, notify::RecursiveMode::NonRecursive)
            .map_err(|e| format!("Error adding lib watch for {:?}: {}", lib_path, e.description()))
    }

    fn library_path(library: &str) -> PathBuf {
        let lib_file = "lib".to_owned() + library + ".so";
        let mut lib_path = PathBuf::from("target");
        #[cfg(not(debug_assertions))]
        lib_path.push("release");
        #[cfg(debug_assertions)]
        lib_path.push("debug");
        lib_path.push(&lib_file);
        lib_path
    }

    pub fn preload_libraries(&mut self, libs: &[&str]) -> Result<(), String>{
        DynamicSystemsLoader::recompile(libs);
        for library in libs {
            let lib_path = Self::library_path(library);
            match self.libraries.entry(lib_path.clone()){
                Entry::Occupied(lib) => (),
                Entry::Vacant(vacant) => {
                    let library = DynamicLibrary(
                        Arc::new(RwLock::new(temporary_library(&lib_path)?))
                    );
                    vacant.insert(library);
                }
            }
        }
        Ok(())
    }

    fn load_library(&mut self, lib_name: &str) -> Result<DynamicLibrary, String>{
        let lib_path = Self::library_path(lib_name);

        let library = match self.libraries.entry(lib_path.clone()){
            Entry::Occupied(lib) => lib.into_mut(),
            Entry::Vacant(vacant) => {
                // Recompile library before first use to ensure that it's up to date
                DynamicSystemsLoader::recompile(&[lib_name]);

                self.library_names_index.entry(lib_path.clone())
                    .or_insert(lib_name.to_owned());

                let library = DynamicLibrary(
                    Arc::new(RwLock::new(temporary_library(&lib_path)?))
                );
                vacant.insert(library)
            }
        };

        Ok(library.clone())
    }

    fn watch_source(&mut self, system_path: &SystemPath) -> Result<(), String>{
        let mut source_path = PathBuf::from("src");
        source_path.push(&system_path.library);
        if source_path.exists() {
            self.source_watcher.watch(&source_path, notify::RecursiveMode::Recursive)
                .map_err(|e| format!("Error adding source watch for {:?}: {}", source_path, e.description()))?;
        }else{
            let mut source_path = PathBuf::from("src");
            source_path.push("systems");
            source_path.push(&system_path.library);
            if source_path.exists() {
                self.source_watcher.watch(&source_path, notify::RecursiveMode::Recursive)
                    .map_err(|e| format!("Error adding source watch for {:?}: {}", source_path, e.description()))?;
            }else{
                println!("Error: couldn't find source for dynamic system {:?}", source_path) // TODO: Panic?
            }
        }
        Ok(())
    }

    fn new_system(&mut self, system_path: &str) -> Result<DynamicGSystem<fn(Entities, Resources)>, String>{
        let system_path = SystemPath::new(system_path)?;

        let library = self.load_library(&system_path.library)?;
        let system = self.systems.new_system(&library, &system_path)?;

        self.watch_source(&system_path)?;

        Ok(system)
    }

    fn new_system_with_data(&mut self, system_path: &str) -> Result<DynamicSystemWithData, String>{
        let system_path = SystemPath::new(system_path)?;

        let library = self.load_library(&system_path.library)?;
        let system = self.systems_with_data.new_system(&library, &system_path)?;

        self.watch_source(&system_path)?;

        Ok(system)
    }

    fn new_system_thread_local(&mut self, system_path: &str) -> Result<DynamicSystemThreadLocal, String>{
        let system_path = SystemPath::new(system_path)?;

        let library = self.load_library(&system_path.library)?;
        let system = self.systems_thread_local.new_system(&library, &system_path)?;

        self.watch_source(&system_path)?;

        Ok(system)
    }

    fn new_system_with_data_thread_local(&mut self, system_path: &str) -> Result<DynamicSystemWithDataThreadLocal, String>{
        let system_path = SystemPath::new(system_path)?;

        let library = self.load_library(&system_path.library)?;
        let system = self.systems_with_data_thread_local.new_system(&library, &system_path)?;

        self.watch_source(&system_path)?;

        Ok(system)
    }

    fn new_creation_system(&mut self, system_path: &str) -> Result<DynamicCreationSystem, String>{
        let system_path = SystemPath::new(system_path)?;

        let library = self.load_library(&system_path.library)?;
        let system = self.creation_systems.new_system(&library, &system_path)?;

        self.watch_source(&system_path)?;

        Ok(system)
    }

    fn new_creation_system_with_data(&mut self, system_path: &str) -> Result<DynamicCreationSystemWithData, String>{
        let system_path = SystemPath::new(system_path)?;

        let library = self.load_library(&system_path.library)?;
        let system = self.creation_systems_with_data.new_system(&library, &system_path)?;

        self.watch_source(&system_path)?;

        Ok(system)
    }

    fn update_source(&mut self){
        for e in self.source_rx.try_iter() {
            match e {
                notify::DebouncedEvent::Write(lib_path) |
                notify::DebouncedEvent::Create(lib_path) => {
                    let library = self.library_names_index
                        .iter()
                        .map(|(_, lib_name)| {
                            let mut source_path1 = current_dir().unwrap();
                            source_path1.push("src");
                            source_path1.push(lib_name);
                            let mut source_path2 = current_dir().unwrap();
                            source_path2.push("src");
                            source_path2.push("systems");
                            source_path2.push(lib_name);
                            (source_path1, source_path2, lib_name)
                        })
                        .find(|&(ref source_path1, ref source_path2, _lib_name)| {
                            lib_path.starts_with(source_path1) || lib_path.starts_with(source_path2)
                        })
                        .map(|(_, _, lib_name)|{
                            lib_name
                        })
                        .unwrap();
                    DynamicSystemsLoader::recompile(&[library]);
                }

                _ => println!("System notify, other event"),
            }
        }
    }

    fn update_libs(&mut self) {
        for e in self.libs_rx.try_iter() {
            match e {
                notify::DebouncedEvent::Write(lib_path) |
                notify::DebouncedEvent::Create(lib_path) => {
                    if let Some(lib_path) = self.libraries
                        .iter()
                        .find(|&(path, _)| lib_path.ends_with(path))
                        .map(|(path, _)| path.clone())
                    {

                        let library = self.libraries.get_mut(&lib_path).unwrap();
                        let mut old_library = library.write().unwrap();
                        let mut unloaded_library = old_library.unload_library();

                        if let Ok((library, templib)) = temporary_library(&lib_path) {
                            let mut new_library = unloaded_library.load(library);

                            self.systems.update(&lib_path, &new_library);
                            self.systems_with_data.update(&lib_path, &new_library);
                            self.systems_thread_local.update(&lib_path, &new_library);
                            self.systems_with_data_thread_local.update(&lib_path, &new_library);
                            self.creation_systems.update(&lib_path, &new_library);
                            self.creation_systems_with_data.update(&lib_path, &new_library);

                            new_library.set_new_library_tempfile(templib);
                        }else{
                            println!("Error: Couldn't reload library {:?}, trying to reload previous library", lib_path);
                            unloaded_library.reload().unwrap();
                        }
                    }
                }

                e => println!("Library notify, other event {:?}", e),
            }
        }
    }
}


pub struct DynamicSystemLoader<S>{
    systems: HashMap<SystemPath, DynamicGSystem<S>>,
    systems_per_library: HashMap<PathBuf, Vec<SystemPath>>,
}

unsafe impl<S> Send for DynamicSystemLoader<S>{}

impl<S> DynamicSystemLoader<S> {
    pub fn new() -> DynamicSystemLoader<S> {
        DynamicSystemLoader{
            systems: HashMap::default(),
            systems_per_library: HashMap::default(),
        }
    }

    fn library_path(library: &str) -> PathBuf {
        let lib_file = "lib".to_owned() + library + ".so";
        let mut lib_path = PathBuf::from("target");
        #[cfg(not(debug_assertions))]
        lib_path.push("release");
        #[cfg(debug_assertions)]
        lib_path.push("debug");
        lib_path.push(&lib_file);
        lib_path
    }

    fn new_system(&mut self, library: &DynamicLibrary, system_path: &SystemPath) -> Result<DynamicGSystem<S>, String> {
        if let Some(system) = self.systems.get(system_path) {
            return Ok(system.clone());
        }

        let lib_path = Self::library_path(&system_path.library);

        let system = library.load_generic_system(system_path)?;

        self.systems.insert(system_path.clone(), system.clone());

        self.systems_per_library.entry(lib_path.clone())
            .or_insert(vec![])
            .push(system_path.clone());

        Ok(system)
    }

    fn update(&mut self, lib_path: &PathBuf, new_library: &DynamicLibraryWriteGuard){
        if let Some(systems) = self.systems_per_library.get_mut(lib_path){
            println!("Reloading {:?} {:?}", lib_path, systems);
            for system_path in systems {
                if let Ok(system) = unsafe{ new_library.get(system_path.system.as_bytes()) } {
                    println!("{}::{} reloaded", system_path.library, system_path.system);
                    self.systems.get_mut(system_path).unwrap().set(system);
                }else{
                    println!("Error: {:?} reloaded but couldn't find system {}", lib_path, system_path.system);
                }
            }
        }
    }
}

pub struct DynamicGSystem<S>{
    library: DynamicLibrary,
    system: Arc<UnsafeCell<libimp::Symbol<S>>> // TODO: do we need a mutex here?
                                                // probably only if we allow to run the
                                                // system from outside the world
}

impl<S> Clone for DynamicGSystem<S>{
    fn clone(&self) -> DynamicGSystem<S>{
        DynamicGSystem {
            library: self.library.clone(),
            system: self.system.clone()
        }
    }
}

impl<S> DynamicGSystem<S>{
    fn set(&mut self, system: libimp::Symbol<S>){
        unsafe{
            let old_system = mem::replace(&mut *self.system.get(), system);
            mem::forget(old_system);
        }
    }
}

unsafe impl<S> Send for DynamicGSystem<S>{}

type DynamicSystem = DynamicGSystem<fn(::Entities, ::Resources)>;
type DynamicSystemWithData = DynamicGSystem<fn(*mut c_void, ::Entities, ::Resources)>;
type DynamicSystemThreadLocal = DynamicGSystem<fn(::EntitiesThreadLocal, ::ResourcesThreadLocal)>;
type DynamicSystemWithDataThreadLocal = DynamicGSystem<fn(*mut c_void, ::EntitiesThreadLocal, ::ResourcesThreadLocal)>;
type DynamicCreationSystem = DynamicGSystem<fn(::EntitiesCreation, ::ResourcesThreadLocal)>;
type DynamicCreationSystemWithData = DynamicGSystem<fn(*mut c_void, ::EntitiesCreation, ::ResourcesThreadLocal)>;


#[derive(Clone)]
struct DynamicLibrary(Arc<RwLock<(lib::Library, tempfile::TempPath)>>);

struct DynamicLibraryReadGuard<'a>(RwLockReadGuard<'a, (lib::Library, tempfile::TempPath)>);
struct DynamicLibraryWriteGuard<'a>(RwLockWriteGuard<'a, (lib::Library, tempfile::TempPath)>);
struct DynamicLibraryWriteGuardUnloaded<'a>(RwLockWriteGuard<'a, (lib::Library, tempfile::TempPath)>);

impl DynamicLibrary{
    fn read(&self) -> Result<DynamicLibraryReadGuard, PoisonError<RwLockReadGuard<(lib::Library, tempfile::TempPath)>>>{
        self.0.read().map(|g| DynamicLibraryReadGuard(g))
    }

    fn write(&self) -> Result<DynamicLibraryWriteGuard, PoisonError<RwLockWriteGuard<(lib::Library, tempfile::TempPath)>>>{
        self.0.write().map(|g| DynamicLibraryWriteGuard(g) )
    }

    fn load_generic_system<S>(&self, system_path: &SystemPath) -> Result<DynamicGSystem<S>, String>{
        let system: libimp::Symbol<S> = unsafe{
            self.read().unwrap().get(system_path.system.as_bytes())
                .map_err(|e| format!("Error loading symbol from {}::{}: {}", system_path.library, system_path.system, e.description()))?
        };
        Ok(DynamicGSystem{
            library: self.clone(),
            system: Arc::new(UnsafeCell::new(system)),
        })
    }
}

impl<'a> DynamicLibraryReadGuard<'a>{
    unsafe fn get<T>(&self, symbol: &[u8]) -> lib::Result<libimp::Symbol<T>>{
        (self.0).0.get(symbol).map(|s: lib::Symbol<T>| s.into_raw())
    }
}

impl<'a> DynamicLibraryWriteGuard<'a>{
    unsafe fn get<T>(&self, symbol: &[u8]) -> lib::Result<libimp::Symbol<T>>{
        (self.0).0.get(symbol).map(|s: lib::Symbol<T>| s.into_raw())
    }

    fn unload_library(mut self) -> DynamicLibraryWriteGuardUnloaded<'a>{
        unsafe{
            mem::replace(&mut (self.0).0, mem::uninitialized());
            DynamicLibraryWriteGuardUnloaded(self.0)
        }
    }

    fn replace_library(&mut self, new_lib: lib::Library){
        mem::replace(&mut (self.0).0, new_lib);
    }

    fn set_new_library_tempfile(&mut self, tempfile: tempfile::TempPath){
        mem::replace(&mut (self.0).1, tempfile);
    }

    fn temp_path(&self) -> &tempfile::TempPath{
        &(self.0).1
    }
}

impl<'a> Drop for DynamicLibraryWriteGuardUnloaded<'a>{
    fn drop(&mut self){
        panic!("Trying to unlock an unloaded dynamic library")
    }
}

impl<'a> DynamicLibraryWriteGuardUnloaded<'a>{
    fn load(mut self, new_lib: lib::Library) -> DynamicLibraryWriteGuard<'a>{
        let old_lib = mem::replace(&mut (self.0).0, new_lib);
        mem::forget(old_lib);
        let guard = unsafe{ mem::replace(&mut self.0, mem::uninitialized()) };
        mem::forget(self);
        DynamicLibraryWriteGuard(guard)
    }

    fn reload(self) -> lib::Result<DynamicLibraryWriteGuard<'a>>{
        let library = lib::Library::new((self.0).1.as_os_str())?;
        Ok(self.load(library))
    }
}

impl<'a> System<'a> for DynamicSystem{
    fn run(&mut self, entities: Entities, resources: Resources) {
        let _lib_lock = self.library.read().unwrap();
        unsafe{(*self.system.get())(entities, resources)}
    }
}

impl<'a, D: Send + 'static> SystemWithData<'a, D> for DynamicSystemWithData{
    fn run(&mut self, data: &mut D, entities: Entities, resources: Resources) {
        let _lib_lock = self.library.read().unwrap();
        unsafe{(*self.system.get())(data as *mut D as *mut c_void, entities, resources)}
    }
}

impl<'a> SystemThreadLocal<'a> for DynamicSystemThreadLocal{
    fn run(&mut self, entities: EntitiesThreadLocal, resources: ResourcesThreadLocal) {
        let _lib_lock = self.library.read().unwrap();
        unsafe{(*self.system.get())(entities, resources)}
    }
}

impl<'a, D: 'static> SystemWithDataThreadLocal<'a, D> for DynamicSystemWithDataThreadLocal{
    fn run(&mut self, data: &mut D, entities: EntitiesThreadLocal, resources: ResourcesThreadLocal) {
        let _lib_lock = self.library.read().unwrap();
        unsafe{(*self.system.get())(data as *mut D as *mut c_void, entities, resources)}
    }
}

impl<'a> CreationSystem<'a> for DynamicCreationSystem{
    fn run(&mut self, entities: EntitiesCreation, resources: ResourcesThreadLocal) {
        let _lib_lock = self.library.read().unwrap();
        unsafe{(*self.system.get())(entities, resources)}
    }
}

impl<'a, D: 'static> CreationSystemWithData<'a, D> for DynamicCreationSystemWithData{
    fn run(&mut self, data: &mut D, entities: EntitiesCreation, resources: ResourcesThreadLocal) {
        let _lib_lock = self.library.read().unwrap();
        unsafe{(*self.system.get())(data as *mut D as *mut c_void, entities, resources)}
    }
}