use std::fs;

use anyhow::Result;

pub struct SystemDService<'a> {
    program_name: &'a str,
    args: Vec<&'a str>,
}

impl<'a> SystemDService<'a> {
    pub fn new(program_name: &'a str, args: Vec<&'a str>) -> Self {
        SystemDService{program_name,args}
    }

    fn create_service_file_string (&self) -> String {
        format!("[Unit]
Description=Part of the data collection package. This is the {} service. 
After=network.target

[Service]
User=root
ExecStart=/usr/local/bin/{} {}
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
",
            self.program_name,
            self.program_name,
            self.args.join(" ")
        )
    }
    
    pub fn create_service_file (&self) -> Result<()> {
        let service_file_string = self.create_service_file_string();
        fs::write(format!("/etc/systemd/system/{}.service", self.program_name), service_file_string)?;

        Ok(())
    }

    pub fn check_program_exists(&self) -> Result<bool> {
        let exists = fs::exists(format!("/usr/local/bin/{}", self.program_name))?;
        Ok(exists)
    }
    pub fn check_service_exists(&self) -> Result<bool> {
        let exists = fs::exists(format!("/etc/systemd/system/{}.service", self.program_name))?;
        Ok(exists)
    }
}


#[cfg(test)]
mod test {
    // use super::*;

    #[test]
    fn should_create_system_file_string() {
        let service = crate::utils::SystemDService::new("hello_world", vec!["-a"]);
        let expected_string = "[Unit]
Description=Part of the data collection package. This is the hello_world service. 
After=network.target

[Service]
User=root
ExecStart=/usr/local/bin/hello_world -a
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
";
        assert_eq!(service.create_service_file_string(), expected_string);
    }
}
