#[derive(Debug, Clone)]
pub struct Config {
    addr: String,
    rdb_dir: String,
    rdb_file: String,
}

impl Config {
    pub fn new(addr: String, rdb_dir: String, rdb_file: String) -> Self {
        Self {
            addr,
            rdb_dir,
            rdb_file,
        }
    }

    pub fn get_addr_string(&self) -> String {
        self.addr.to_string()
    }

    pub fn get_rdb_file(&self) -> String {
        self.rdb_file.to_string()
    }

    pub fn get_rdb_dir(&self) -> String {
        self.rdb_dir.to_string()
    }
}
