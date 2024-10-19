mod class_file;

use anyhow::{anyhow, Error};
use std::collections::HashMap;
use std::env::{args, current_dir};
use std::fs;
use std::fs::File;
use std::rc::Rc;
use zip::ZipArchive;

pub fn run() -> Result<(), Error> {
    let mut classes = HashMap::new();

    let jar_dir = current_dir()?.join("data");

    for path in fs::read_dir(jar_dir)? {
        let path = path?.path();
        let zip_reader = File::open(path)?;
        let mut zip_archive = ZipArchive::new(zip_reader)?;

        let class_files: Vec<String> = zip_archive.file_names()
            .filter(|file| file.ends_with(".class"))
            .map(|str| str.to_string())
            .collect();

        for file in class_files {
            let file = zip_archive.by_name(&file)?;
            let class_file = class_file::read_class_file(file)?;

            insert_class(&mut classes, class_file)?;
        }
    }

    let main_class_name = args().nth(1).ok_or(anyhow!("required main class"))?;
    let main_class = classes.get(&main_class_name).ok_or(anyhow!("unknown class {}", main_class_name))?;

    let main_method = main_class.methods.iter()
        .find(|method| method.name.eq("main") && method.descriptor.eq("([Ljava/lang/String;)V"))
        .ok_or(anyhow!("can't find main method"))?;

    let mut thread = create_thread(main_method);

    run_thread(&mut thread)?;

    Ok(())
}

#[derive(Debug)]
struct RuntimeClass {
    this_class: String,
    methods: Vec<Method2>,
}

#[derive(Debug)]
struct Method2 {
    name: String,
    descriptor: String,
    code: class_file::Code,
}

fn insert_class(classes: &mut HashMap<String, Rc<RuntimeClass>>, class_file: class_file::ClassFile) -> Result<Rc<RuntimeClass>, Error> {
    let class_name = class_file.const_pool.get((class_file.this_class - 1) as usize).ok_or(anyhow!("error"))?;

    let class_name = match class_name {
        class_file::Const::Class { name_idx } => {
            let class_name = class_file.const_pool.get((name_idx.clone() - 1) as usize).ok_or(anyhow!("error"))?;
            match class_name {
                class_file::Const::Utf8 { bytes } => bytes,
                _ => return Err(anyhow!("expected utf8, not {:?}", class_name))
            }
        }
        _ => return Err(anyhow!("expected class, not {:?}", class_name))
    };

    let mut methods = Vec::with_capacity(class_file.methods.len());
    for method in class_file.methods {
        let name = class_file.const_pool.get((method.name_idx - 1) as usize).ok_or(anyhow!("error"))?;
        let name = match name {
            class_file::Const::Utf8 { bytes } => bytes,
            _ => return Err(anyhow!("error"))
        };

        let descriptor = class_file.const_pool.get((method.descriptor_idx - 1) as usize).ok_or(anyhow!("error"))?;
        let descriptor = match descriptor {
            class_file::Const::Utf8 { bytes } => bytes,
            _ => return Err(anyhow!("error"))
        };

        methods.push(Method2 {
            name: name.to_string(),
            descriptor: descriptor.to_string(),
            code: method.code,
        });
    }

    let class = Rc::new(RuntimeClass {
        this_class: class_name.to_string(),
        methods,
    });

    classes.insert(class.this_class.clone().replace("/", "."), class.clone());

    Ok(class)
}

struct Thread {
    frames: Vec<Frame>,
}

struct Frame {
    pc: usize,
    code: Vec<u8>,
}

fn create_thread(method: &Method2) -> Thread {
    Thread {
        frames: vec![Frame {
            pc: 0,
            code: method.code.code.clone(),
        }],
    }
}

fn run_thread(thread: &mut Thread) -> Result<(), Error> {
    while let Some(frame) = thread.frames.last_mut() {
        while frame.pc < frame.code.len() {
            let instr = frame.code[frame.pc];
            match instr {
                0xB1 => {
                    thread.frames.pop();
                    break;
                }
                _ => Err(anyhow!("unknown instruction {:#02x}", instr))?
            }
        }
    }
    Ok(())
}
