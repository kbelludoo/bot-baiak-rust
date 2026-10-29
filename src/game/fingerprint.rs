use std::fs;
use uuid::Uuid;

pub const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
pub const DEVICE_FILE_NAME: &str = "baiak_device";

pub fn djb2_hex(input: &str) -> String {
    let mut t: u32 = 5381;
    for b in input.bytes() {
        t = ((t << 5).wrapping_add(t)).wrapping_add(b as u32);
    }
    format!("{:08x}", t)
}

pub fn generate_device_id() -> String {
    Uuid::new_v4().simple().to_string()
}

pub fn get_or_create_device_id(account_id: &str) -> String {
    let dir = format!("data/{}", account_id);
    let path = format!("{}/{}", dir, DEVICE_FILE_NAME);
    if let Ok(content) = fs::read_to_string(&path) {
        let trimmed = content.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }

    let dev_id = generate_device_id();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(&path, &dev_id);
    dev_id
}

pub fn build_fingerprint(account_id: &str) -> String {
    let dev_id = get_or_create_device_id(account_id);
    let clean_id = if dev_id.len() > 32 { &dev_id[..32] } else { &dev_id };
    let raw = format!("{}|1920|1080|24|-180|pt-BR", DEFAULT_USER_AGENT);
    format!("{}-{}", clean_id, djb2_hex(&raw))
}
