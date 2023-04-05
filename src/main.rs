use elf_rs::*;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::metadata;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::str;
use clap::Parser;


/// Search for libraries in a directory ELF files and display all found dependencies on libraries in a directory.
#[derive(Parser)]
struct Cli {
    // "The path to the directory to read"
    path: Option<std::path::PathBuf>,
}

struct ElfCounter {
    arc_files: HashMap<String, Vec<String>>,
    n_files: u32,
}


fn get_architecture(file_name: &String, directory: &String) -> ElfMachine {
    let md = metadata(directory).unwrap();
    let mut path = PathBuf::new();

    if md.is_dir() {
        path.push(directory);
    } else {
        path = env::current_dir()
            .expect("There are insufficient permissions to access the current directory");
        path.push(directory);
    }
    path.push(file_name);

    let md = metadata(&path).unwrap();
    if md.is_dir() {
        return ElfMachine::MachineUnknown(1);
    } else {
        let mut elf_file = File::open(&path).expect("open file failed");
        let mut elf_buf = Vec::<u8>::new();
        elf_file
            .read_to_end(&mut elf_buf)
            .expect("read file failed");
        match Elf::from_bytes(&mut elf_buf) {
            Ok(elf_data) => {
                return elf_data.elf_header().machine();
            }
            Err(_e) => {
                return ElfMachine::MachineUnknown(0);
            }
        }
    }
}

fn scan_dir(directory: &String) -> Vec<String> {
    let paths = fs::read_dir(directory).unwrap();
    let names = paths
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str().map(|s| String::from(s)))
            })
        })
        .collect::<Vec<String>>();
    return names;
}

fn collect_lib<'a>(
    names: &Vec<String>,
    directory: &String,
    lib_map: &'a mut HashMap<String, ElfCounter>,
) {
    for file in names.iter() {
        let output = Command::new("ldd")
            .arg(&file)
            .current_dir(&directory)
            .output()
            .expect("failed to execute process");

        let s = match str::from_utf8(&output.stdout) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };

        let architecture = get_architecture(&file, &directory);
        if architecture == ElfMachine::MachineUnknown(0) {
            continue;
        }
        if architecture == ElfMachine::MachineUnknown(1) {
            let mut path = String::new();
            path.push_str(directory);
            path.push_str("/");
            path.push_str(file);

            let names = scan_dir(&path);
            collect_lib(&names, &path, lib_map);
            continue;
        }

        let key = format!("{architecture:?}");

        for lib in s.split('\n') {
            match &mut lib_map.get_mut(&lib.to_string()) {
                Some(entry) => match entry.arc_files.get_mut(&key) {
                    Some(arc_map) => {
                        arc_map.push(file.to_string());
                        entry.n_files = entry.n_files + 1;
                    }
                    None => {
                        let mut list_file_names: Vec<String> = Vec::new();
                        list_file_names.push(file.to_string());
                        let mut arc_files: HashMap<String, Vec<String>> = HashMap::new();
                        arc_files.insert(key.clone(), list_file_names);
                    }
                },
                None => {
                    if lib ==""{
                        continue;
                    }
                    let mut list_file_names: Vec<String> = Vec::new();
                    list_file_names.push(file.to_string());
                    let mut arc_files: HashMap<String, Vec<String>> = HashMap::new();
                    arc_files.insert(key.clone(), list_file_names);
                    let elf_counter = ElfCounter {
                        arc_files: arc_files,
                        n_files: 1,
                    };
                    lib_map.insert(lib.to_string(), elf_counter);
                }
            }
        }
    }
}

fn main() {
    let args = Cli::from_args();
    if args.path.is_none(){
       return;
    }

    let args: Vec<String> = env::args().collect();
    let dir = &args[1];

    // let dir = &Cli::path;
    let mut lib_map: HashMap<String, ElfCounter> = HashMap::new();
    let names = scan_dir(&dir);
    collect_lib(&names, &dir, &mut lib_map);

    let mut vec: Vec<_> = lib_map.iter().collect();
    vec.sort_by(|(_k, v), (_k2, v2)| v2.n_files.cmp(&v.n_files));


    for (lib, elf_counter) in vec {
        println!("Lib name {}", lib);
        println!("{:_<1$}n_exec({2})", "", 30, elf_counter.n_files);
        for (arc, file_container) in &elf_counter.arc_files {
            println!("{2}{:_<1$} ", "", 10, arc);
            for file in file_container {
                println!("{}\n\n", file);
            }
        }
    }
}
