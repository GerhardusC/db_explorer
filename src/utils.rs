use std:: fs;

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

fn download_release(save_name: &str) -> Result<()> {
    let res = reqwest::blocking::get("https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip")?;
    let body = res.bytes()?;
    fs::write(save_name, body)?;

    Ok(())
}

fn unzip_file(archive_name: &str, target_dir: &str) -> Result<String> {
    let _ = fs::create_dir(target_dir);

    let res = std::process::Command::new("unzip")
        .args(vec![archive_name, "-d", target_dir])
        .output()?;

    let stdout = String::from_utf8(res.stdout)?;
    let stderr = String::from_utf8(res.stderr)?;
    
    Ok(format!("{}, {}", stdout, stderr))
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn should_download_zip_file() {
        let save_name = "./release.zip";
        let downloaded = download_release(&save_name);
        assert!(downloaded.is_ok());

        assert!(fs::exists(&save_name)
            .expect("Should be able to call exists on a file")
        );

        fs::remove_file(&save_name)
            .expect("Unable to remove file created in test");
    }

    #[test]
    fn should_unzip_file() {
        let target_dir = "./temp";
        let archive_name = "./release.zip";
        assert!(fs::exists(archive_name)
            .expect("Should be able to call exists on file")
        );
        let result = unzip_file(archive_name, target_dir)
            .expect("Should be able to unzip file");

        assert!(result.contains("inflating"));
        // cleanup
        let _ = fs::remove_dir_all(target_dir);
    }

    #[test]
    fn should_create_system_file_string() {
        let service = SystemDService::new("hello_world", vec!["-a"]);
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

