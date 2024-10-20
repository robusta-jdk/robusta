use anyhow::{anyhow, Error};
use std::io::Read;

impl ClassFile {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, Error> {
        // Skip unused fields
        read_length(reader, 8)?;

        let const_pool = ConstPool::from_reader(reader)?;

        read_u16(reader)?; // access flags
        let this_class = read_u16(reader)?;
        read_u16(reader)?; // super class

        let interface_count = read_u16(reader)?;
        for _ in 0..interface_count {
            read_u16(reader)?; // interface
        }

        let field_count = read_u16(reader)?;
        for _ in 0..field_count {
            read_u16(reader)?; // access flags
            read_u16(reader)?; // name index
            read_u16(reader)?; // descriptor index
            let attribute_count = read_u16(reader)?; // attribute count
            for _ in 0..attribute_count {
                read_u16(reader)?; // name index
                let length = read_u32(reader)?;
                read_length(reader, length as usize)?;
            }
        }

        let method_count = read_u16(reader)?;
        let mut methods = Vec::with_capacity(method_count as usize);
        for _ in 0..method_count {
            methods.push(Method::from_reader(reader)?);
        }

        let attribute_count = read_u16(reader)?; // attribute count
        let mut attributes = Vec::with_capacity(attribute_count as usize);
        for _ in 0..attribute_count {
            attributes.push(Attribute::from_reader(reader)?);
        }

        Ok(ClassFile {
            const_pool,
            this_class,
            methods,
            _attributes: attributes,
        })
    }
}

#[derive(Debug)]
pub struct ClassFile {
    pub const_pool: ConstPool,
    pub this_class: u16,
    pub methods: Vec<Method>,
    pub _attributes: Vec<Attribute>,
}

#[derive(Debug)]
pub struct ConstPool {
    consts: Vec<Const>,
}

impl ConstPool {
    fn from_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let const_pool_count = read_u16(reader)?;
        let mut pool = Vec::with_capacity(const_pool_count as usize - 1);
        for _ in 1..const_pool_count {
            pool.push(read_const(reader)?);
        }
        Ok(ConstPool { consts: pool })
    }

    pub fn get_utf8(&self, idx: u16) -> Result<&Utf8, Error> {
        let const_item = self.get_const(idx)?;
        match const_item {
            Const::Utf8(utf8) => Ok(utf8),
            _ => Err(anyhow!("expected utf8, got {:?}", const_item))
        }
    }

    pub fn get_class(&self, idx: u16) -> Result<&Class, Error> {
        let const_item = self.get_const(idx)?;
        match const_item {
            Const::Class(class) => Ok(class),
            _ => Err(anyhow!("expected class, got {:?}", const_item))
        }
    }

    fn get_const(&self, idx: u16) -> Result<&Const, Error> {
        self.consts.get(idx as usize - 1).ok_or(anyhow!("const pool does not have an item at index {}", idx))
    }
}

#[derive(Debug, PartialEq)]
pub enum Const {
    Utf8(Utf8),
    Class(Class),
    Unimplemented,
}

#[derive(Debug, PartialEq)]
pub struct Utf8 {
    pub bytes: String,
}

#[derive(Debug, PartialEq)]
pub struct Class {
    pub name_idx: u16,
}

#[derive(Debug)]
pub struct Method {
    pub name_idx: u16,
    pub descriptor_idx: u16,
    pub attributes: Vec<Attribute>,
}

impl Method {
    fn from_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let _access_flags = read_u16(reader)?;
        let name_idx = read_u16(reader)?;
        let descriptor_idx = read_u16(reader)?;
        let attributes_count = read_u16(reader)?;
        let mut attributes = Vec::with_capacity(attributes_count as usize);
        for _ in 0..attributes_count {
            attributes.push(Attribute::from_reader(reader)?);
        }
        Ok(Self { name_idx, descriptor_idx, attributes })
    }
}

#[derive(Debug)]
pub struct Attribute {
    pub name_idx: u16,
    pub info: Vec<u8>,
}

impl Attribute {
    fn from_reader<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let name_idx = read_u16(reader)?;
        let length = read_u32(reader)?;
        let info = read_length(reader, length as usize)?;
        Ok(Self { name_idx, info })
    }
}

#[derive(Debug)]
pub struct Code {
    pub _max_stack: u16,
    pub _max_locals: u16,
    pub code: Vec<u8>,
}

impl Code {
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self, Error> {
        let max_stack = read_u16(reader)?;
        let max_locals = read_u16(reader)?;
        let code_length = read_u32(reader)?;
        let code = read_length(reader, code_length as usize)?;
        let ex_table_length = read_u16(reader)?;
        let _ex_table = read_length(reader, ex_table_length as usize * 8)?;
        let attributes_length = read_u16(reader)?;
        for _ in 0..attributes_length {
            Attribute::from_reader(reader)?;
        }
        Ok(Self { _max_stack: max_stack, _max_locals: max_locals, code })
    }
}

fn read_u8<R: Read>(reader: &mut R) -> Result<u8, Error> {
    let mut bytes = [0; 1];
    reader.read_exact(&mut bytes)?;
    Ok(bytes[0])
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32, Error> {
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_be_bytes(bytes))
}

fn read_u16<R: Read>(reader: &mut R) -> Result<u16, Error> {
    let mut bytes = [0; 2];
    reader.read_exact(&mut bytes)?;
    Ok(u16::from_be_bytes(bytes))
}

fn read_length<R: Read>(reader: &mut R, length: usize) -> Result<Vec<u8>, Error> {
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod read_length_tests {
    use super::*;

    const BYTES: [u8; 5] = [0x10, 0x20, 0x30, 0x40, 0x50];
    const EMPTY: [u8; 0] = [];

    fn reader() -> &'static [u8] {
        &BYTES as &[u8]
    }

    fn empty() -> &'static [u8] {
        &EMPTY as &[u8]
    }

    #[test]
    fn test_read_u8_ok() {
        let result = read_u8(&mut reader());

        assert_eq!(result.unwrap(), 0x10);
    }

    #[test]
    fn test_read_u8_err() {
        let result = read_u8(&mut empty());

        assert!(result.is_err());
    }

    #[test]
    fn test_read_u16_ok() {
        let result = read_u16(&mut reader());

        assert_eq!(result.unwrap(), 0x1020);
    }

    #[test]
    fn test_read_u16_err() {
        let result = read_u16(&mut empty());

        assert!(result.is_err());
    }

    #[test]
    fn test_read_u32_ok() {
        let result = read_u32(&mut reader());

        assert_eq!(result.unwrap(), 0x10203040);
    }

    #[test]
    fn test_read_u32_err() {
        let result = read_u32(&mut empty());

        assert!(result.is_err());
    }

    #[test]
    fn test_read_length_ok() {
        let result = read_length(&mut reader(), 3);

        assert_eq!(result.unwrap(), vec![0x10, 0x20, 0x30]);
    }

    #[test]
    fn test_read_length_err() {
        let result = read_length(&mut empty(), 3);

        assert!(result.is_err());
    }
}

fn read_const<R: Read>(reader: &mut R) -> Result<Const, Error> {
    let tag = read_u8(reader)?;
    match tag {
        1 => {
            let length = read_u16(reader)?;
            let bytes = read_length(reader, length as usize)?;
            Ok(Const::Utf8(Utf8 { bytes: String::from_utf8(bytes)? }))
        }
        7 => {
            let name_idx = read_u16(reader)?;
            Ok(Const::Class(Class { name_idx }))
        }
        10 | 12 => {
            read_u32(reader)?;
            Ok(Const::Unimplemented)
        }
        _ => Err(anyhow!("Unimplemented tag {}", tag))
    }
}

#[cfg(test)]
mod read_const_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_utf8_ok() {
        let reader: Vec<u8> = vec![
            vec![0x01],
            ("hello world".len() as u16).to_be_bytes().into(),
            "hello world".bytes().collect()
        ].into_iter().flatten().collect();

        let utf8_const = read_const(&mut Cursor::new(reader));

        assert_eq!(utf8_const.unwrap(), Const::Utf8(Utf8 { bytes: "hello world".to_string() }));
    }

    #[test]
    fn read_utf8_err() {
        let reader = vec![0x01, 0x0, 0x2];

        let utf8_const = read_const(&mut Cursor::new(reader));

        assert!(utf8_const.is_err());
    }

    #[test]
    fn read_class_ok() {
        let reader = vec![0x07, 0x23, 0x45];

        let utf8_const = read_const(&mut Cursor::new(reader));

        assert_eq!(utf8_const.unwrap(), Const::Class(Class { name_idx: 0x2345 }));
    }

    #[test]
    fn read_class_err() {
        let reader = vec![0x07];

        let utf8_const = read_const(&mut Cursor::new(reader));

        assert!(utf8_const.is_err());
    }
}
