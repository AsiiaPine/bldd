use clap::Parser;
use elf_rs::*;
use regex::Regex;
use std::env;
use std::fs;
use std::fs::metadata;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::str;



/// Search for libraries in a directory ELF files and display all found dependencies on libraries in a directory.
#[derive(Parser)]
#[clap(author="Anastasiia Stepanova <asiiapien@gmail.com>", version, about="Search for libraries in a directory ELF files and display all found dependencies on libraries in a directory.")]
struct Cli {
    /// The path to the directory to read
    path: String,

    /// Recursive traversal of directory
    #[arg(
        short, long)]
    recursive: bool,
}

#[derive(Debug, Clone)]
struct CollectorEntry {
    file_path: String,
    lib_name: String,
    machine_arc: ElfMachine,
}

/// Groups items in a vector by applying a lambda to each item to determine its
/// group key. Returns a vector of tuples where the first element is the group
/// key and the second element is a vector of all items in that group.
///
/// # Examples
///
/// ```
/// let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
/// let groups = group_by(numbers, |n| n % 3);
/// assert_eq!(groups, [(1, [1, 4, 7, 10]), (2, [2, 5, 8]), (0, [3, 6, 9])]);
/// ```
///
/// # Arguments
///
/// * `vec`: A vector of items to group.
///
/// * `f`: A closure that accepts a reference to an item in the vector and returns
///        a key to group the item by.
///
/// # Returns
///
/// A vector of tuples where the first element is the group key and the second
/// element is a vector of all items in that group.
fn group_by<T, K, F>(items: Vec<T>, key_func: F) -> Vec<(K, Vec<T>)>
where
    T: Clone,
    F: Fn(&T) -> K,
    K: Eq,
{
    let mut result: Vec<(K, Vec<T>)> = Vec::new();

    for item in items {
        let key = key_func(&item);
        let mut found = false;

        for group in result.iter_mut() {
            if group.0 == key {
                group.1.push(item.clone());
                found = true;
                break;
            }
        }

        if !found {
            result.push((key, vec![item.clone()]));
        }
    }

    result
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
    collector: &'a mut Vec<CollectorEntry>,
    recursive: &bool
) {
    for file in names.iter() {
        let mut path = String::new();
        path.push_str(directory);
        path.push_str("/");
        path.push_str(file);

        let output = Command::new("readelf")
            .arg("-d")
            .arg(&file)
            .current_dir(&directory)
            .output()
            .expect("failed to execute process");

        let s = match str::from_utf8(&output.stdout) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        // Get architecture of the file
        let architecture = get_architecture(&file, &directory);
        if architecture == ElfMachine::MachineUnknown(0) {
            continue;
        }
        if architecture == ElfMachine::MachineUnknown(1) && recursive.clone(){
            let names = scan_dir(&path);
            collect_lib(&names, &path, collector, recursive);
            continue;
        }
        // Extraction of lib from readelf output line
        let re = Regex::new(r"\[([^\]]+)\]").unwrap();
        for lib in s.split('\n').filter(|x| x.contains("Shared library:")) {
            // If the lib extraction passed successfully:
            if let Some(captures) = re.captures(lib) {
                let library = captures.get(1).unwrap().as_str();

                let result = CollectorEntry {
                    lib_name: library.to_string(),
                    file_path: path.clone(),
                    machine_arc: architecture,
                };
                collector.push(result);
            }
        }
    }
}


fn main() {
    let args = Cli::parse();
    let dir = args.path;

    let mut collector: Vec<CollectorEntry> = Vec::new();
    let names = scan_dir(&dir);
    collect_lib(&names, &dir, &mut collector, &args.recursive);

    let vec = group_by(collector, |a: &CollectorEntry| a.machine_arc);
    for (machine_type, entries) in vec {
        println!("\n\n---------- {:?} ----------", machine_type);
        let mut vec2 = group_by(entries, |x| x.lib_name.clone());
        vec2.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
        for (lib_name, entries2) in vec2 {
            println!("  {} ({} executables) ->", lib_name, entries2.len());
            for entry in entries2 {
                println!("    {}", entry.file_path);
            }
            println!();
        }
    }
}
