use std::fs;
use systemdzbus::{Connection, manager::ManagerProxy};

use anyhow::Result;
use tokio::task::JoinHandle;
/**
Example of git_repo This is how to get latest release:
"https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".
Git repo is expected to be a release. */
pub struct SystemDService<'a> {
    github_release_link: &'a str,
    service_name: &'a str,
    program_name: &'a str,
    startup_args: Vec<&'a str>,
}

impl<'a> SystemDService<'a> {
    pub fn new(
        git_repo: &'a str,
        service_name: &'a str,
        program_name: &'a str,
        startup_args: Vec<&'a str>,
    ) -> Self {
        SystemDService {
            github_release_link: git_repo,
            service_name,
            program_name,
            startup_args,
        }
    }

    fn create_unit_file_string(&self) -> String {
        format!(
            "[Unit]
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
            self.startup_args.join(" ")
        )
    }

    pub fn create_unit_file(&self) -> Result<()> {
        let service_file_string = self.create_unit_file_string();
        fs::write(
            format!("/etc/systemd/system/{}.service", self.service_name),
            service_file_string,
        )?;

        Ok(())
    }

    pub fn check_program_exists(&self) -> Result<bool> {
        let exists = fs::exists(format!("/usr/local/bin/{}", self.program_name))?;
        Ok(exists)
    }
    pub fn check_unit_file_exists(&self) -> Result<bool> {
        let exists = fs::exists(format!("/etc/systemd/system/{}.service", self.service_name))?;
        Ok(exists)
    }

    pub async fn check_unit_registered(&self) -> Result<bool> {
        let connection = Connection::system().await?;

        let proxy = ManagerProxy::new(&connection).await?;

        let result = proxy.get_unit(&format!("{}.service", self.service_name)).await;
        match result {
            Ok(res) => {
                println!("Service is running: {:?}", res);
                Ok(true)
            },
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("NoSuchUnit") {
                    return Ok(false)
                }
                return Err(e.into());
            },
        }
    }

    pub fn download_release(&self) -> Result<()> {
        let res = reqwest::blocking::get(self.github_release_link)?;
        let body = res.bytes()?;
        fs::write(&format!("./{}.zip", self.service_name), body)?;

        Ok(())
    }

    pub fn unzip_file(&self, target_dir: &str) -> Result<String> {
        let archive_name = format!("./{}.zip", self.service_name);
        let _ = fs::create_dir(target_dir);

        let res = std::process::Command::new("unzip")
            .args(vec!["-o", &archive_name, "-d", target_dir])
            .output()?;

        let stdout = String::from_utf8(res.stdout)?;
        let stderr = String::from_utf8(res.stderr)?;

        Ok(format!("{}, {}", stdout, stderr))
    }
}

#[cfg(test)]
mod test {
    use std::process;

    use super::*;

    #[test]
    fn should_check_unit_registered() {
        let rt = tokio::runtime::Builder::new_current_thread().build()
            .expect("Should prepare async runtime for test");

        let res: Result<bool> = rt.block_on(async {
            let service = SystemDService::new(
                "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip",
                "substore",
                "sub_store",
                vec![],
            );

            let res = service.check_unit_registered().await?;

            Ok(res)
        });

        assert!(res.is_ok());
        assert!(!res.expect("should be able to check unit file"));

    }

    // Ignoring this test for now to ensure we don't keep downloading the file.
    #[ignore]
    #[test]
    fn should_download_zip_file() {
        let service = SystemDService::new(
            "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip",
            "substore",
            "sub_store",
            vec![],
        );

        let downloaded = service.download_release();
        assert!(downloaded.is_ok());

        assert!(
            fs::exists(&format!("./{}.zip", service.service_name))
                .expect("Should be able to call exists on a file")
        );

        fs::remove_file(&format!("./{}.zip", service.service_name))
            .expect("Unable to remove file created in test");
    }

    #[test]
    fn should_unzip_file() {
        // SETUP:
        let service = SystemDService::new(
            "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip",
            "substore",
            "sub_store",
            vec![],
        );

        // - mock setup -
        let dummy_file_name = "./sub_store";
        let dummy_zipped_name = "./substore.zip";
        let target_dir = "./temp";

        // Create dummy file to zip.
        fs::write(dummy_file_name, "hello")
            .expect("Should be able to create dummy file");

        // Zip up dummy file
        let _new_zip = process::Command::new("zip")
            .args(vec![dummy_zipped_name, dummy_file_name])
            .output()
            .expect("Should be able to create dummy zip file");

        assert!(fs::exists(dummy_zipped_name).expect("Should be able to call exists on file"));

        // PERFORM
        let result = service
            .unzip_file(target_dir)
            .expect("Should be able to unzip file");

        // ASSERT
        assert!(result.contains("extracting"));

        // CLEANUP
        let _ = fs::remove_dir_all(target_dir);
        let _ = fs::remove_file(dummy_zipped_name);
        let _ = fs::remove_file(dummy_file_name);
    }

    #[test]
    fn should_create_unit_file_string() {
        let service = SystemDService::new(
            "",
            "service_name",
            "program_name",
            vec!["-a"]
        );
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
        assert_eq!(service.create_unit_file_string(), expected_string);
    }
}
