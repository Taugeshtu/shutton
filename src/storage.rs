use std::fs::File;
use std::io::{Read, Write, Seek, SeekFrom};

const MAGIC: &[u8; 8] = b"SHUTTNC2";

#[derive(Clone, Debug)]
pub struct Config {
    pub autoquit: bool,
    pub width: i32,
    pub main_cmd: String,
    pub var_values: Vec<String>,
    pub description: String,
    pub active_fold: u8,
}

pub fn read_config() -> Option<Config> {
    let exe_path = std::env::current_exe().ok()?;
    let mut file = File::open(&exe_path).ok()?;
    let file_len = file.metadata().ok()?.len();
    if file_len < 22 {
        return None;
    }
    
    // Read magic signature
    file.seek(SeekFrom::End(-8)).ok()?;
    let mut magic_buf = [0u8; 8];
    file.read_exact(&mut magic_buf).ok()?;
    if &magic_buf != MAGIC {
        return None;
    }
    
    // Read autoquit (1 byte)
    file.seek(SeekFrom::End(-9)).ok()?;
    let mut aq_buf = [0u8; 1];
    file.read_exact(&mut aq_buf).ok()?;
    let autoquit = aq_buf[0] != 0;
    
    // Read active_fold (1 byte)
    file.seek(SeekFrom::End(-10)).ok()?;
    let mut af_buf = [0u8; 1];
    file.read_exact(&mut af_buf).ok()?;
    let active_fold = af_buf[0];
    
    // Read width (4 bytes)
    file.seek(SeekFrom::End(-14)).ok()?;
    let mut w_buf = [0u8; 4];
    file.read_exact(&mut w_buf).ok()?;
    let width = i32::from_le_bytes(w_buf);
    
    // Read number of strings
    file.seek(SeekFrom::End(-18)).ok()?;
    let mut num_buf = [0u8; 4];
    file.read_exact(&mut num_buf).ok()?;
    let num_strings = u32::from_le_bytes(num_buf) as usize;
    
    if num_strings < 2 {
        return None;
    }
    
    // Read lengths
    let offset_lengths = 18 + (num_strings * 4) as i64;
    file.seek(SeekFrom::End(-offset_lengths)).ok()?;
    let mut lengths = Vec::new();
    for _ in 0..num_strings {
        let mut len_buf = [0u8; 4];
        file.read_exact(&mut len_buf).ok()?;
        lengths.push(u32::from_le_bytes(len_buf) as usize);
    }
    
    // Read strings
    let total_str_len: usize = lengths.iter().sum();
    let offset_strs = offset_lengths + total_str_len as i64;
    file.seek(SeekFrom::End(-offset_strs)).ok()?;
    
    let mut strings = Vec::new();
    for &len in &lengths {
        let mut str_buf = vec![0u8; len];
        file.read_exact(&mut str_buf).ok()?;
        let s = String::from_utf8(str_buf).ok()?;
        strings.push(s);
    }
    
    if strings.len() < 2 {
        return None;
    }
    
    let main_cmd = strings.remove(0);
    let description = strings.remove(0);
    
    Some(Config {
        autoquit,
        width,
        main_cmd,
        description,
        active_fold,
        var_values: strings,
    })
}

pub fn patch_config(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = std::env::current_exe()?;
    let mut file_bytes = std::fs::read(&exe_path)?;
    
    let file_len = file_bytes.len();
    let mut payload_len = 0;
    if file_len >= 18 {
        let magic_start = file_len - 8;
        if &file_bytes[magic_start..] == MAGIC {
            let num_start = file_len - 18;
            let mut num_bytes = [0u8; 4];
            num_bytes.copy_from_slice(&file_bytes[num_start..num_start + 4]);
            let num_strings = u32::from_le_bytes(num_bytes) as usize;
            
            let lengths_len = num_strings * 4;
            if file_len >= 18 + lengths_len {
                let lengths_start = file_len - 18 - lengths_len;
                let mut total_str_len = 0;
                for i in 0..num_strings {
                    let idx = lengths_start + i * 4;
                    let mut len_bytes = [0u8; 4];
                    len_bytes.copy_from_slice(&file_bytes[idx..idx + 4]);
                    total_str_len += u32::from_le_bytes(len_bytes) as usize;
                }
                
                if file_len >= 18 + lengths_len + total_str_len {
                    payload_len = 18 + lengths_len + total_str_len;
                }
            }
        }
    }
    
    if payload_len > 0 {
        file_bytes.truncate(file_len - payload_len);
    }
    
    let mut strings = vec![config.main_cmd.clone(), config.description.clone()];
    strings.extend(config.var_values.clone());
    
    let mut string_bytes = Vec::new();
    let mut lengths = Vec::new();
    for s in &strings {
        let bytes = s.as_bytes();
        string_bytes.extend_from_slice(bytes);
        lengths.push(bytes.len() as u32);
    }
    
    file_bytes.extend_from_slice(&string_bytes);
    
    for len in lengths {
        file_bytes.extend_from_slice(&len.to_le_bytes());
    }
    
    let num_strings = strings.len() as u32;
    file_bytes.extend_from_slice(&num_strings.to_le_bytes());
    
    file_bytes.extend_from_slice(&config.width.to_le_bytes());
    
    file_bytes.push(config.active_fold);
    
    let aq_byte = if config.autoquit { 1u8 } else { 0u8 };
    file_bytes.push(aq_byte);
    
    file_bytes.extend_from_slice(MAGIC);
    
    let tmp_path = exe_path.with_extension("tmp_patch");
    {
        let mut tmp_file = File::create(&tmp_path)?;
        tmp_file.write_all(&file_bytes)?;
    }
    
    let metadata = std::fs::metadata(&exe_path)?;
    std::fs::set_permissions(&tmp_path, metadata.permissions())?;
    
    std::fs::rename(&tmp_path, &exe_path)?;
    Ok(())
}
