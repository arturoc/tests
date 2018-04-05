use libloading as lib;
use std::path::PathBuf;
use std::ffi::CString;
use std::time::Duration;
use std::mem;
#[cfg(unix)]
use libloading::os::unix as libimp;
#[cfg(windows)]
use libloading::os::windows as libimp;
use std::process::Command;
use std::error::Error;
use notify::{self, Watcher};
use std::sync::mpsc::{channel, Receiver};
use fxhash::FxHashMap as HashMap;
use std::collections::hash_map::Entry;
use ::System;
use ::Entities;
use ::Resources;
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard, PoisonError};
use std::ops::Deref;
use std::cell::UnsafeCell;
use std::thread;
use std::env::current_dir;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use tempfile;


#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct SystemPath{
    library: String,
    system: String,
}

struct Data{
    libraries: HashMap<PathBuf, DynamicLibrary>,
    systems: HashMap<SystemPath, DynamicSystem>,
    library_names_index: HashMap<PathBuf, String>, //TODO: this can probably be a vector or use src path as index instead
    systems_per_library: HashMap<PathBuf, Vec<SystemPath>>,
    source_rx: Receiver<notify::DebouncedEvent>,
    source_watcher: notify::RecommendedWatcher,
    libs_rx: Receiver<notify::DebouncedEvent>,
    libs_watcher: notify::RecommendedWatcher,
    done: bool,
}

pub struct DynamicSystemLoader{
    data: Arc<Mutex<Data>>,
    // updater: thread::JoinHandle,
}

impl DynamicSystemLoader{
    pub fn new() -> Result<DynamicSystemLoader,String>{
        let data = Arc::new(Mutex::new(Data::new()?));
        let loader_data = data.clone();
        let updater = thread::spawn(move ||{
            DynamicSystemLoader::update(data);
        });
        Ok(DynamicSystemLoader{
            data: loader_data,
            // updater,
        })
    }

    pub fn new_system(&mut self, system_path: &str) -> Result<DynamicSystem, String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .new_system(system_path)
    }

    pub fn start(&mut self) -> Result<(), String>{
        self.data.lock()
            .map_err(|e| format!("Couldn't lock dynamic system loader: {}", e.description()))?
            .start()
    }

    fn recompile(library: &str) {
        println!("Recompiling {:?}", library);
        match Command::new("cargo")
            .args(&["build", "--release", "-p", library])
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

            for e in data.source_rx.try_iter() {
                match e {
                    notify::DebouncedEvent::Write(lib_path) |
                    // notify::DebouncedEvent::NoticeWrite(lib_path) |
                    notify::DebouncedEvent::Create(lib_path) => {
                        let library = data.library_names_index
                            .iter()
                            .map(|(_, lib_name)| {
                                let mut source_path = current_dir().unwrap();
                                source_path.push("src");
                                source_path.push(lib_name);
                                (source_path, lib_name)
                            })
                            .find(|&(ref source_path, _lib_name)| {
                                lib_path.starts_with(source_path)
                            })
                            .map(|(_, lib_name)|{
                                lib_name
                            })
                            .unwrap();
                        DynamicSystemLoader::recompile(library);
                    }

                    _ => println!("System notify, other event"),
                }
            }

            let systems_per_library = unsafe{ mem::transmute::<
                &mut HashMap<PathBuf, Vec<SystemPath>>,
                &mut HashMap<PathBuf, Vec<SystemPath>>>(&mut data.systems_per_library)};
            let libraries = unsafe{ mem::transmute::<
                &mut HashMap<PathBuf, DynamicLibrary>,
                &mut HashMap<PathBuf, DynamicLibrary>>(&mut data.libraries) };
            let data_systems = unsafe{ mem::transmute::<
                &mut HashMap<SystemPath, DynamicSystem>,
                &mut HashMap<SystemPath, DynamicSystem>>(&mut data.systems) };
            for e in data.libs_rx.try_iter() {
                match e {
                    notify::DebouncedEvent::Write(lib_path) |
                    // notify::DebouncedEvent::NoticeWrite(lib_path) |
                    // notify::DebouncedEvent::Chmod(lib_path) |
                    notify::DebouncedEvent::Create(lib_path) => {
                        if let Some(lib_path) = data.libraries
                            .iter()
                            .find(|&(path, _)| lib_path.ends_with(path))
                            .map(|(path, _)| path)
                        {
                            let systems = systems_per_library.get_mut(lib_path).unwrap();
                            println!("Reloading {:?} {:?}", lib_path, systems);
                            let library = libraries.get_mut(lib_path).unwrap();
                            let mut old_library = library.write().unwrap();
                            let mut unloaded_library = old_library.unload_library();

                            if let Ok((library, templib)) = temporary_library(&lib_path) {
                                let mut new_library = unloaded_library.load(library);
                                for system_path in systems {
                                    if let Ok(system) = unsafe{ new_library.get(system_path.system.as_bytes()) } {
                                        println!("{}::{} reloaded", system_path.library, system_path.system);
                                        data_systems.get_mut(system_path).unwrap().set(system);
                                    }else{
                                        println!("Error: {:?} reloaded but couldn't find system {}", lib_path, system_path.system);
                                    }
                                }
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
            systems: HashMap::default(),
            library_names_index: HashMap::default(),
            systems_per_library: HashMap::default(),
            source_watcher,
            libs_watcher,
            source_rx,
            libs_rx,
            done: false,
        })
    }

    fn start(&mut self) -> Result<(), String>{
        // let mut lib_path = PathBuf::from("target");

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

    fn new_system(&mut self, system_path: &str) -> Result<DynamicSystem, String>{

        let mut system_parts = system_path.split("::");
        let lib_name = system_parts.next().expect("Empty system path");
        let system_name = system_parts.next()
            .expect(&format!("Can't find system in path with only one part {}, system paths have to be specified as library::system", system_path));
        assert_eq!(system_parts.next(), None);

        let system_path = SystemPath{
            library: lib_name.to_owned(),
            system: system_name.to_owned(),
        };

        if let Some(system) = self.systems.get(&system_path) {
            return Ok(system.clone());
        }

        let lib_file = "lib".to_owned() + lib_name + ".so";
        let mut lib_path = PathBuf::from("target");
        lib_path.push("release");
        lib_path.push(&lib_file);

        // let os_path = OsStr::new(lib_path.as_ref().to_str().unwrap());
        let library = match self.libraries.entry(lib_path.clone()){
            Entry::Occupied(lib) => lib.into_mut(),
            Entry::Vacant(vacant) => {
                // Recompile library before first use to ensure that it's up to date
                DynamicSystemLoader::recompile(lib_name);

                let library = DynamicLibrary(
                    Arc::new(RwLock::new(temporary_library(&lib_path)?))
                );
                vacant.insert(library)
            }
        };

        let system: libimp::Symbol<fn(Entities, Resources)> = unsafe{
            library.read().unwrap().get(system_name.as_bytes())
                .map_err(|e| format!("Error loading symbol from {}::{}: {}", system_path.library, system_path.system, e.description()))?
        };
        let system = DynamicSystem{
            library: library.clone(),
            system: Arc::new(UnsafeCell::new(Box::new(system))),
        };
        self.systems.insert(system_path.clone(), system.clone());

        self.library_names_index.entry(lib_path.clone())
            .or_insert(lib_name.to_owned());

        self.systems_per_library.entry(lib_path.clone())
            .or_insert(vec![])
            .push(system_path.clone());

        let mut source_path = PathBuf::from("src");
        source_path.push(lib_name);
        self.source_watcher.watch(&source_path, notify::RecursiveMode::Recursive)
            .map_err(|e| format!("Error adding source watch for {:?}: {}", source_path, e.description()))?;
        Ok(system)
    }
}

#[derive(Clone)]
pub struct DynamicSystem{
    library: DynamicLibrary,
    system: Arc<UnsafeCell<Box<for<'a> System<'a>>>> // TODO: do we need a mutex here?
                                                // probably only if we allow to run the
                                                // system from outside the world
}

impl DynamicSystem{
    fn set<S: 'static + for<'a> System<'a>>(&mut self, system: S){
        unsafe{
            mem::replace(&mut *self.system.get(), Box::new(system));
        }
    }
}

unsafe impl Send for DynamicSystem{}

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

// impl Deref for DynamicLibrary{
//     type Target = Arc<RwLock<(lib::Library, tempfile::TempPath)>>;
//     fn deref(&self) -> &Arc<RwLock<(lib::Library, tempfile::TempPath)>>{
//         &self.0
//     }
// }

impl<'a> System<'a> for DynamicSystem{
    fn run(&mut self, entities: Entities, resources: Resources) {
        let _lib_lock = self.library.read().unwrap();
        unsafe{(*self.system.get()).run(entities, resources)}
    }
}

impl<'a> System<'a> for libimp::Symbol<fn(Entities, Resources)> {
    fn run(&mut self, entities: Entities, resources: Resources) {
        self(entities, resources)
    }
}
