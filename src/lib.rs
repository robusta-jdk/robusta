use anyhow::{anyhow, format_err, Error};
use std::collections::HashMap;
use std::env::{args, current_dir};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use zip::read::ZipFile;
use zip::ZipArchive;

pub fn run() -> Result<(), Error> {
    let mut classes = HashMap::new();

    let jar_dir = current_dir()?.join("data");

    for path in fs::read_dir(jar_dir)? {
        let path = path?.path();
        let zip_reader = File::open(path)?;
        let mut zip_Rch = ZipArchive::new(zip_reader)?;

        let class_files: Vec<String> = zip_Rch.file_names()
            .filter(|file| file.ends_with(".class"))
            .map(|str| str.to_string())
            .collect();

        for file in class_files {
            let file = zip_Rch.by_name(&file)?;
            let class_file = read_class_file(file)?;
            println!("class file {:?}", &class_file);

            insert_class(&mut classes, class_file)?;
        }
    }

    println!("Ready to run");

    let main_class_name = args().nth(1).ok_or(anyhow!("required main class"))?;
    let main_class = classes.get(&main_class_name).ok_or(anyhow!("unknown class {}", main_class_name))?;

    let main_method = main_class.methods.iter()
        .find(|method| method.name.eq("main") && method.descriptor.eq("([Ljava/lang/String;)V"))
        .ok_or(anyhow!("can't find main method"))?;

    println!("got main method {:?}", main_method.code.code);

    Ok(())
}

fn read_class_file(mut file: ZipFile) -> Result<ClassFile, Error> {
    let mut class_file = ClassFile {
        const_pool: vec![],
        this_class: 0,
        super_class: 0,
        methods: vec![],
    };

    read_u32(&mut file)?; // magic
    read_u16(&mut file)?; // minor
    read_u16(&mut file)?; // major

    let const_pool_count = read_u16(&mut file)?;
    class_file.const_pool = Vec::with_capacity(const_pool_count as usize);
    for _ in 1..const_pool_count {
        class_file.const_pool.push(read_const(&mut file)?);
    }

    read_u16(&mut file)?; // access flags
    class_file.this_class = read_u16(&mut file)?;
    class_file.super_class = read_u16(&mut file)?;

    let interface_count = read_u16(&mut file)?;
    for _ in 0..interface_count {
        read_u16(&mut file)?; // interface
    }

    let field_count = read_u16(&mut file)?;
    for _ in 0..field_count {
        read_u16(&mut file)?; // access flags
        read_u16(&mut file)?; // name index
        read_u16(&mut file)?; // descriptor index
        let attribute_count = read_u16(&mut file)?; // attribute count
        for _ in 0..attribute_count {
            read_u16(&mut file)?; // name index
            let length = read_u32(&mut file)?;
            read_length(&mut file, length as usize)?;
        }
    }

    let method_count = read_u16(&mut file)?;
    class_file.methods = Vec::with_capacity(method_count as usize);
    for _ in 0..method_count {
        let mut method = Method {
            name_idx: 0,
            descriptor_idx: 0,
            code: Code {
                max_stack: 0,
                max_locals: 0,
                code: vec![],
            },
        };

        read_u16(&mut file)?; // access flags
        method.name_idx = read_u16(&mut file)?;
        method.descriptor_idx = read_u16(&mut file)?;

        let attribute_count = read_u16(&mut file)?;
        for _ in 0..attribute_count {
            let name_idx = read_u16(&mut file)?;

            let const_item = class_file.const_pool.get((name_idx - 1) as usize).ok_or(format_err!("expected const pool item"))?;
            if let Const::Utf8 { bytes } = const_item {
                if bytes.eq("Code") {
                    read_u32(&mut file)?; // length
                    let max_stack = read_u16(&mut file)?;
                    let max_locals = read_u16(&mut file)?;
                    let code_length = read_u32(&mut file)?;
                    let code = read_length(&mut file, code_length as usize)?;

                    // ex table
                    let ex_length = read_u16(&mut file)?;
                    read_length(&mut file, ex_length as usize * 8)?;

                    let attribute_count = read_u16(&mut file)?; // attribute count
                    for _ in 0..attribute_count {
                        read_u16(&mut file)?; // name index
                        let length = read_u32(&mut file)?;
                        read_length(&mut file, length as usize)?;
                    }
                    method.code.max_stack = max_stack;
                    method.code.max_locals = max_locals;
                    method.code.code = code;
                } else {
                    let length = read_u32(&mut file)?;
                    read_length(&mut file, length as usize)?;
                }
            } else {
                return Err(format_err!("expected utf8"));
            }
        }
        class_file.methods.push(method);
    }

    let attribute_count = read_u16(&mut file)?; // attribute count
    for _ in 0..attribute_count {
        read_u16(&mut file)?; // name index
        let length = read_u32(&mut file)?;
        read_length(&mut file, length as usize)?;
    }

    Ok(class_file)
}

#[derive(Debug)]
struct ClassFile {
    const_pool: Vec<Const>,
    this_class: u16,
    super_class: u16,
    methods: Vec<Method>,
}

#[derive(Debug)]
enum Const {
    Utf8 { bytes: String },
    Class { name_idx: u16 },
    Unimplemented,
}

#[derive(Debug)]
struct Method {
    name_idx: u16,
    descriptor_idx: u16,
    code: Code,
}

#[derive(Debug)]
struct Code {
    max_stack: u16,
    max_locals: u16,
    code: Vec<u8>,
}

fn read_u8(file: &mut ZipFile) -> Result<u8, Error> {
    let mut bytes = [0; 1];
    file.read(&mut bytes)?;
    Ok(bytes[0])
}

fn read_u32(file: &mut ZipFile) -> Result<u32, Error> {
    let mut bytes = [0; 4];
    file.read(&mut bytes)?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_u16(file: &mut ZipFile) -> Result<u16, Error> {
    let mut bytes = [0; 2];
    file.read(&mut bytes)?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_length(file: &mut ZipFile, length: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = vec![0; length];
    file.read(&mut bytes)?;
    Ok(bytes)
}

fn read_const(file: &mut ZipFile) -> Result<Const, Error> {
    let tag = read_u8(file)?;
    match tag {
        1 => {
            let length = read_u16(file)?;
            let bytes = read_length(file, length as usize)?;
            Ok(Const::Utf8 { bytes: String::from_utf8(bytes)? })
        }
        5 | 6 => {
            read_u32(file)?;
            read_u32(file)?;
            Ok(Const::Unimplemented)
        }
        7 => {
            let name_idx = read_u16(file)?;
            Ok(Const::Class { name_idx })
        }
        8 | 16 => {
            read_u16(file)?;
            Ok(Const::Unimplemented)
        }
        3 | 4 | 9 | 10 | 11 | 12 | 18 => {
            read_u32(file)?;
            Ok(Const::Unimplemented)
        }
        15 => {
            read_u8(file)?;
            read_u16(file)?;
            Ok(Const::Unimplemented)
        }
        _ => Err(anyhow!("Unimplemented tag {}", tag))
    }
}

#[derive(Debug)]
struct RuntimeClass {
    this_class: String,
    super_class: Option<Rc<RuntimeClass>>,
    methods: Vec<Method2>,
}

#[derive(Debug)]
struct Method2 {
    name: String,
    descriptor: String,
    code: Code,
}

fn insert_class(classes: &mut HashMap<String, Rc<RuntimeClass>>, class_file: ClassFile) -> Result<Rc<RuntimeClass>, Error> {
    let class_name = class_file.const_pool.get((class_file.this_class - 1) as usize).ok_or(anyhow!("error"))?;

    let class_name = match class_name {
        Const::Class { name_idx } => {
            let class_name = class_file.const_pool.get((name_idx.clone() - 1) as usize).ok_or(anyhow!("error"))?;
            match class_name {
                Const::Utf8 { bytes } => bytes,
                _ => return Err(anyhow!("expected utf8, not {:?}", class_name))
            }
        }
        _ => return Err(anyhow!("expected class, not {:?}", class_name))
    };

    let mut methods = Vec::with_capacity(class_file.methods.len());
    for method in class_file.methods {
        let name = class_file.const_pool.get((method.name_idx - 1) as usize).ok_or(anyhow!("error"))?;
        let name = match name {
            Const::Utf8 { bytes } => bytes,
            _ => return Err(anyhow!("error"))
        };

        let descriptor = class_file.const_pool.get((method.descriptor_idx - 1) as usize).ok_or(anyhow!("error"))?;
        let descriptor = match descriptor {
            Const::Utf8 { bytes } => bytes,
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
        super_class: None,
        methods: methods,
    });

    classes.insert(class.this_class.clone(), class.clone());

    Ok(class)
}
