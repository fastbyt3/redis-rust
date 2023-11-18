#[derive(Debug, Clone)]
pub struct Config {
    addr: String,
    rdb_dir: Option<String>,
    rdb_file: Option<String>,
}

impl Config {
    pub fn new(addr: String, rdb_dir: Option<String>, rdb_file: Option<String>) -> Self {
        Self {
            addr,
            rdb_dir,
            rdb_file,
        }
    }

    pub fn get_addr_string(&self) -> String {
        self.addr.to_string()
    }

    pub fn get_rdb_file(&self) -> Option<String> {
        self.rdb_file.clone()
    }

    pub fn get_rdb_dir(&self) -> Option<String> {
        self.rdb_dir.clone()
    }

    pub fn get_rdb_path(&self) -> Option<String> {
        match self.rdb_dir.clone() {
            Some(dir) => match self.rdb_file.clone() {
                Some(fname) => Some(format!("{}/{}", dir, fname)),
                None => None,
            },
            None => None,
        }
    }
}
