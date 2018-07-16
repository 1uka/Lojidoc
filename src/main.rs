extern crate clap;
extern crate regex;
extern crate threadpool;

mod model;
mod parse;

use clap::App;
use clap::Arg;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use threadpool::ThreadPool;

use model::model::Class;
use model::model::LineType;
use parse::parse::parse_file;

fn is_java_file(file: &str) -> bool {
    let line_vec: Vec<&str> = file.split(".").collect::<Vec<&str>>();
    let l_index = line_vec.len() - 1;

    if line_vec[l_index].contains("java") {
        true
    } else {
        false
    }
}

/// Traverses the file structure to find all java files for parsing.
///
/// # Arguments
///
/// * `start_dir` - The directory to start looking for java files in.
pub fn find_java_files(start_dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();

    for f in fs::read_dir(start_dir).unwrap() {
        let p = f.unwrap().path();

        if p.is_dir() {
            let path = p.as_path();
            let new_files = find_java_files(path);

            for n_file in new_files {
                files.push(n_file.clone());
            }
        } else if p.is_file() {
            if is_java_file(p.as_path().file_name().unwrap().to_str().unwrap()) {
                files.push(p.clone());
            }
        }
    }

    files.clone()
}

/// Generates a markdown file for a java file
/// Uses a Class struct to write the markdown
///
/// # Arguments
///
/// * `class` - The class struct containing the java documentation data
/// * `dest` - The file path where the markdown file will be saved
pub fn generate_markdown(class: Class, dest: &str) {
    let name = format!("{}/{}.{}", dest, class.class_name, "md");
    let mut file = File::create(name).unwrap();

    let mut doc = format!("# {}\n\n", class.class_name);

    if class.description.as_str() != "" {
        doc.push_str(format!("description: {}\n", class.description.trim()).as_str());
    }
    doc.push_str(format!("privacy: {}\n", class.access.trim()).as_str());
    doc.push_str(format!("package: {}\n\n", class.package_name.trim()).as_str());
    doc.push_str("## Dependencies\n\n");

    for dep in class.dependencies {
        doc.push_str(format!("- {}\n", dep).as_str());
    }
    doc.push_str("\n## Methods\n\n");

    for member in class.methods {
        doc.push_str(format!("#### {}\n\n", member.name).as_str());
        doc.push_str(format!("privacy: {}\n", member.privacy.trim()).as_str());
        doc.push_str(format!("description: {}\n", member.description).as_str());
        doc.push_str(format!("return: {}\n\n", member.return_type).as_str());

        if member.parameters.len() > 0 {
            doc.push_str("| Name | Type | Description |\n|_____|_____|_____|\n");
        } else {
            doc.push_str("This method has no parameters.\n");
        }

        for param in member.parameters {
            doc.push_str(format!("| {} | {} | {} |\n", param.name, param.var_type, param.desc).as_str());
        }

        doc.push_str("\n");
    }

    file.write(doc.as_str().as_bytes())
        .expect("Not able to write to file");
    println!("{}.{} was created", class.class_name, "md");
}

/// Handles the thread pooling the application
///
/// # Arguments
///
/// * `file_paths` - A vector of the file paths of java files
/// * `dest` - The file path where the markdown will be saved
pub fn document(file_paths: Vec<PathBuf>, dest: String) {
    let files = Arc::new(file_paths);
    let size = files.len();
    let mut pool_size = size / 4;
    if files.len() % 4 != 0 {
        pool_size += 1;
    }
    let pool = ThreadPool::new(pool_size);
    let safe_dest = Arc::new(dest);

    for i in 0..pool_size {
        let file_cp = files.clone();
        let new_dest = safe_dest.clone();

        pool.execute(move || {
            for j in 0..3 {
                if (i * 4) + j < size {
                    let class = parse_file(&file_cp[(i * 4) + j]);
                    generate_markdown(class, new_dest.as_str());
                }
            }
        });
    }

    pool.join();
}

fn main() {
    let matches = App::new("Javadoc-To-Markdown")
        .version("1.0")
        .author("Josh Brudnak <jobrud314@gmail.com>")
        .about("A tool for generating markdown documentation from javadocs")
        .arg(
            Arg::with_name("INPUT")
                .value_name("FILE")
                .required(true)
                .help("Sets the input directory to use")
                .index(1),
        )
        .arg(
            Arg::with_name("context")
                .help("Sets the context path of the project")
                .short("c"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .help("Generate verbose documentation for a project"),
        )
        .arg(
            Arg::with_name("destination")
                .required(false)
                .value_name("FILE")
                .short("d")
                .help("Sets the destination directory of the created markdown files"),
        )
        .get_matches();

    let dir = matches
        .value_of("INPUT")
        .expect("Documentation directory not chosen")
        .to_string();
    let dest = matches
        .value_of("destination")
        .unwrap_or("./generated/")
        .to_string();

    fs::create_dir_all(dest.as_str()).expect("File path not able to be created");
    println!("Generating documentation from {}", dir);

    let file_paths = find_java_files(Path::new(dir.clone().as_str()));

    if file_paths.len() > 0 {
        document(file_paths, dest);
    } else {
        println!("No java files found");
    }
}
