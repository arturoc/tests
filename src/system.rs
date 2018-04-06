pub trait System<'a>: Send{
    fn run(&mut self, ::Entities<'a>, ::Resources<'a>);
}

pub trait SystemThreadLocal<'a>{
    fn run(&mut self, ::EntitiesThreadLocal<'a>, ::ResourcesThreadLocal<'a>);
}

impl<'a, F: FnMut(::Entities<'a>, ::Resources<'a>) + Send> System<'a> for F{
    fn run(&mut self, e: ::Entities<'a>, r: ::Resources<'a>){
        (*self)(e,r)
    }
}

impl<'a, F: FnMut(::EntitiesThreadLocal<'a>, ::ResourcesThreadLocal<'a>)> SystemThreadLocal<'a> for F{
    fn run(&mut self, e: ::EntitiesThreadLocal<'a>, r: ::ResourcesThreadLocal<'a>){
        (*self)(e,r)
    }
}

pub trait SystemResources<'a>{
    fn run(&mut self, ::Resources<'a>);
}

impl<'a, F: FnMut(::Resources<'a>)> SystemResources<'a> for F{
    fn run(&mut self, e: ::Resources<'a>){
        (*self)(e)
    }
}

pub trait SystemEntities<'a>{
    fn run(&mut self, ::Entities<'a>);
}

impl<'a, F: FnMut(::Entities<'a>)> SystemEntities<'a> for F{
    fn run(&mut self, e: ::Entities<'a>){
        (*self)(e)
    }
}

pub trait SystemWithSettings: for<'a> System<'a> + for<'a> SystemSettingsReturn<'a>{
    type Settings;
    fn new(settings: Self::Settings) -> Self;
    fn from_boxed_settings(settings: Box<Self::Settings>) -> Box<for<'a> SystemSettingsReturn<'a>>;
    fn settings(&self) -> Self::Settings;
}

pub trait SystemSettingsReturn<'a>: System<'a>{
    fn settings_boxed(&self) -> Box<::std::os::raw::c_void>;
}

impl<'a, S: SystemWithSettings> SystemSettingsReturn<'a> for S{
    fn settings_boxed(&self) -> Box<::std::os::raw::c_void>{
        unsafe{
            Box::from_raw(Box::into_raw(Box::new(self.settings())) as *mut ::std::os::raw::c_void)
        }
    }
}

pub trait SystemWithSettingsThreadLocal: for<'a> SystemThreadLocal<'a> + for<'a> SystemSettingsReturnThreadLocal<'a>{
    type Settings;
    fn new(settings: Self::Settings) -> Self;
    fn from_boxed_settings(settings: Box<Self::Settings>) -> Box<for<'a> SystemSettingsReturnThreadLocal<'a>>;
    fn settings(&self) -> Self::Settings;
}

pub trait SystemSettingsReturnThreadLocal<'a>: SystemThreadLocal<'a>{
    fn settings_boxed(&self) -> Box<::std::os::raw::c_void>;
}

impl<'a, S: SystemWithSettingsThreadLocal> SystemSettingsReturnThreadLocal<'a> for S{
    fn settings_boxed(&self) -> Box<::std::os::raw::c_void>{
        unsafe{
            Box::from_raw(Box::into_raw(Box::new(self.settings())) as *mut ::std::os::raw::c_void)
        }
    }
}


// #[cfg(feature="dynamic_systems")]
// mod dynamic_system {
//     use libloading as lib;
//     use std::path::PathBuf;
//     use std::ffi::CString;
//     use std::time::Duration;
//     use std::mem;
//     #[cfg(unix)]
//     use libloading::os::unix as libimp;
//     #[cfg(windows)]
//     use libloading::os::windows as libimp;
//     use std::process::Command;
//     use std::error::Error;
//     use notify::{self, Watcher};
//     use std::sync::mpsc::{channel, Receiver};


//     pub struct SystemReload{
//         library: lib::Library,
//         system: libimp::Symbol<fn(::Entities, ::Resources)>,
//         lib_path: PathBuf,
//         system_path: String,
//         system_name: String,
//         rx_lib: Receiver<notify::DebouncedEvent>,
//         rx_source: Receiver<notify::DebouncedEvent>,
//         _libwatcher: notify::RecommendedWatcher,
//         _sourcewatcher: notify::RecommendedWatcher,

//     }

//     impl SystemReload{
//         pub fn new(system_path: &str) -> Result<SystemReload, String>{
//             let mut system_parts = system_path.split("::");
//             let lib_name = system_parts.next().expect("Empty system path");
//             let system_name = system_parts.next().expect(&format!("Can't find system in path with only one part {}, system paths have to be specified as library::system", system_path));
//             assert_eq!(system_parts.next(), None);

//             let lib_file = "lib".to_owned() + lib_name + ".so";
//             let mut lib_path = PathBuf::from("target");
//             lib_path.push("release");
//             lib_path.push(&lib_file);

//             // let os_path = OsStr::new(lib_path.as_ref().to_str().unwrap());
//             let library = lib::Library::new(&lib_path).map_err(|e| e.description().to_owned())?;
//             let system = unsafe{
//                 let system: lib::Symbol<fn(::Entities, ::Resources)> = library.get(system_name.as_bytes())
//                     .map_err(|e| format!("Error loading symbol from {}: {}", system_path, e.description()))?;
//                 system.into_raw()
//             };

//             let (tx, rx_lib) = channel();
//             let mut _libwatcher: notify::RecommendedWatcher = notify::Watcher::new(tx, Duration::from_secs(1))
//                 .map_err(|e| format!("Error creating lib wathcer for {:?}: {}", lib_path, e.description()))?;
//             _libwatcher.watch(&lib_path, notify::RecursiveMode::Recursive)
//                 .map_err(|e| format!("Error adding lib watch for {:?}: {}", lib_path, e.description()))?;

//             let mut source_path = PathBuf::from("src");
//             source_path.push(lib_name);
//             let (tx, rx_source) = channel();
//             let mut _sourcewatcher: notify::RecommendedWatcher = notify::Watcher::new(tx, Duration::from_secs(1))
//                 .map_err(|e| format!("Error creating source watcher for {:?}: {}", source_path, e.description()))?;
//             _sourcewatcher.watch(&source_path, notify::RecursiveMode::Recursive)
//                 .map_err(|e| format!("Error adding source watch for {:?}: {}", source_path, e.description()))?;
//             Ok(
//                 SystemReload{
//                     lib_path,
//                     system_path: system_path.to_owned(),
//                     system_name: system_name.to_owned(),
//                     library,
//                     system,
//                     _libwatcher,
//                     rx_lib,
//                     _sourcewatcher,
//                     rx_source,
//                 }
//             )
//         }
//     }

//     impl<'s> super::System<'s> for SystemReload{
//         fn run(&mut self, e: ::Entities<'s>, r: ::Resources<'s>){
//             match self.rx_source.try_iter().last() {
//                 Some(_) => match Command::new("cargo")
//                     .args(&["build", "--release", "-p", "trafo_update"])
//                     .output(){
//                         Ok(output) => {
//                             let stderr = CString::new(output.stderr).unwrap().into_string().unwrap();
//                             let stdout = CString::new(output.stdout).unwrap().into_string().unwrap();
//                             if output.status.success() {
//                                 println!("Built succesfully {}\n{}", stdout, stderr);
//                             }else{
//                                 println!("Error building system {}\n{}", stdout, stderr);
//                             }
//                         }
//                         Err(err) => println!("Error building system {}", err.description()),
//                 },

//                 None => ()
//             }


//             match self.rx_lib.try_iter().last() {
//                 Some(_) => {
//                     unsafe{
//                         // drop old library to avoid dlopen refcount
//                         mem::replace(&mut self.library, mem::uninitialized());
//                     }
//                     if let Ok(library) = lib::Library::new(&self.lib_path) {
//                         if let Ok(system) = unsafe{
//                             let system: lib::Result<lib::Symbol<fn(::Entities, ::Resources)>> = library.get(self.system_name.as_bytes());
//                             system.map(|s| s.into_raw())
//                         }{
//                             println!("{} reloaded", self.system_path);
//                             // self.last_modified = last_modified;
//                             self.system = system;
//                             let old_library = mem::replace(&mut self.library, library);
//                             mem::forget(old_library);
//                         }else{
//                             println!("Error: {:?} reloaded but couldn't find system {}", self.lib_path, self.system_name);
//                         }
//                     }else{
//                         println!("Error: Couldn't reload library {:?}", self.lib_path);
//                     }
//                 }

//                 None => ()
//             }

//             (*self.system)(e, r);
//         }
//     }
// }


// #[cfg(feature="dynamic_systems")]
// pub use self::dynamic_system::SystemReload;
