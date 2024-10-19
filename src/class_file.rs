use std::io::Read;
use anyhow::{anyhow, format_err, Error};
use zip::read::ZipFile;

pub fn read_class_file(mut file: ZipFile) -> Result<ClassFile, Error> {
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
pub struct ClassFile {
    pub const_pool: Vec<Const>,
    pub this_class: u16,
    pub super_class: u16,
    pub methods: Vec<Method>,
}

#[derive(Debug)]
pub enum Const {
    Utf8 { bytes: String },
    Class { name_idx: u16 },
    Unimplemented,
}

#[derive(Debug)]
pub struct Method {
    pub name_idx: u16,
    pub descriptor_idx: u16,
    pub code: Code,
}

#[derive(Debug)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
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