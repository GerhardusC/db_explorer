use std:: fs;
use systemdzbus::{ Connection ,manager::ManagerProxy };

use anyhow::Result;
/** 
 Example of git_repo This is how to get latest release:
 "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".
 Git repo is expected to be a release. */
pub struct SystemDService<'a> {
    github_release_link: &'a str,
    service_name: &'a str,
    program_name: &'a str,
    args: Vec<&'a str>,
}

impl<'a> SystemDService<'a> {
    pub fn new(git_repo: &'a str, service_name: &'a str, program_name: &'a str, args: Vec<&'a str>) -> Self {
        SystemDService{github_release_link: git_repo, service_name, program_name ,args}
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
            self.service_name,
            self.program_name,
            self.args.join(" ")
        )
    }
    
    pub fn create_service_file (&self) -> Result<()> {
        let service_file_string = self.create_service_file_string();
        fs::write(format!("/etc/systemd/system/{}.service", self.service_name), service_file_string)?;

        Ok(())
    }

    pub fn check_program_exists(&self) -> Result<bool> {
        let exists = fs::exists(format!("/usr/local/bin/{}", self.program_name))?;
        Ok(exists)
    }
    pub fn check_service_file_exists(&self) -> Result<bool> {
        let exists = fs::exists(format!("/etc/systemd/system/{}.service", self.service_name))?;
        Ok(exists)
    }

    pub fn download_release(&self) -> Result<()> {
        let res = reqwest::blocking::get(self.github_release_link)?;
        let body = res.bytes()?;
        fs::write(&format!("./{}.zip", self.service_name), body)?;

        Ok(())
    }
}

// TODO: Also move this into impl block above.
fn unzip_file(archive_name: &str, target_dir: &str) -> Result<String> {
    let _ = fs::create_dir(target_dir);

    let res = std::process::Command::new("unzip")
        .args(vec!["-o", archive_name, "-d", target_dir])
        .output()?;

    let stdout = String::from_utf8(res.stdout)?;
    let stderr = String::from_utf8(res.stderr)?;
    
    Ok(format!("{}, {}", stdout, stderr))
}


#[cfg(test)]
mod test {
    use std::process;

    use super::*;

    #[test]
    fn should_download_zip_file() {
        let service = SystemDService::new(
            "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip",
            "substore",
            "sub_store",
            vec![]
        );

        let downloaded = service.download_release();
        assert!(downloaded.is_ok());

        assert!(fs::exists(&format!("./{}.zip", service.service_name))
            .expect("Should be able to call exists on a file")
        );

        fs::remove_file(&format!("./{}.zip", service.service_name))
            .expect("Unable to remove file created in test");
    }

    #[test]
    fn should_unzip_file() {
        let dummy_file_name = "./text.txt";
        let dummy_zipped_name = "./release.zip";

        // Create dummy file to zip.
        fs::write(dummy_file_name, "hello")
            .expect("Should be able to create dummy file");

        let _new_zip = process::Command::new("zip")
            .args(vec![dummy_zipped_name, dummy_file_name])
            .output()
            .expect("Should be able to create dummy zip file");

        let target_dir = "./temp";

        assert!(fs::exists(dummy_zipped_name)
            .expect("Should be able to call exists on file")
        );
        let result = unzip_file(dummy_zipped_name, target_dir)
            .expect("Should be able to unzip file");

        assert!(result.contains("extracting"));

        // cleanup
        let _ = fs::remove_dir_all(target_dir);
        let _ = fs::remove_file(dummy_zipped_name);
        let _ = fs::remove_file(dummy_file_name);
    }

    #[test]
    fn should_create_system_file_string() {
        let service = SystemDService::new("", "service_name", "program_name", vec!["-a"]);
        let expected_string = "[Unit]
Description=Part of the data collection package. This is the service_name service. 
After=network.target

[Service]
User=root
ExecStart=/usr/local/bin/program_name -a
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
";
        assert_eq!(service.create_service_file_string(), expected_string);
    }
}

