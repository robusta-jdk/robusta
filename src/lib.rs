mod class_file;

use crate::class_file::{ClassFile, Code};
use anyhow::{anyhow, Error};
use std::collections::HashMap;
use std::env::{args, current_dir};
use std::fs;
use std::fs::File;
use std::io::Cursor;
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
            let mut file = zip_archive.by_name(&file)?;
            let class_file = ClassFile::read_from(&mut file)?;
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
    methods: Vec<RuntimeMethod>,
}

// Need to think about how we name this
#[derive(Debug)]
struct RuntimeMethod {
    name: String,
    descriptor: String,
    code: Code,
}

fn insert_class(classes: &mut HashMap<String, Rc<RuntimeClass>>, class_file: ClassFile) -> Result<Rc<RuntimeClass>, Error> {
    let this_class = class_file.const_pool.get_class(class_file.this_class)?;
    let class_name = class_file.const_pool.get_utf8(this_class.name_idx)?;

    let mut methods = Vec::with_capacity(class_file.methods.len());
    for method in class_file.methods {
        let name = class_file.const_pool.get_utf8(method.name_idx)?;
        let descriptor = class_file.const_pool.get_utf8(method.descriptor_idx)?;

        let code_attr = method.attributes.iter().find(|attr| {
            class_file.const_pool.get_utf8(attr.name_idx).ok().map(|name_const| {
                name_const.bytes.eq("Code")
            }).unwrap_or_else(|| false)
        });

        let code = if let Some(code_attr) = code_attr {
            let mut reader = Cursor::new(&code_attr.info);
            Code::read_from(&mut reader)?
        } else {
            Code {
                _max_stack: 0,
                _max_locals: 0,
                code: vec![],
            }
        };

        methods.push(RuntimeMethod {
            name: name.bytes.clone(),
            descriptor: descriptor.bytes.clone(),
            code,
        });
    }

    let class = Rc::new(RuntimeClass {
        this_class: class_name.bytes.clone(),
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

fn create_thread(method: &RuntimeMethod) -> Thread {
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
