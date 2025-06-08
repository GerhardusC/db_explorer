use std::{fs::{self, Permissions}, os::unix::fs::PermissionsExt, path::Path};
use systemdzbus::{manager::ManagerProxy, Connection};

use anyhow::Result;

pub enum ProgramKind {
    DataCollector,
    ServerProgram,
}

/**
Example of git_repo This is how to get latest release:
"https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".
Git repo is expected to be a release. */
#[derive(Clone)]
pub struct SystemDService {
    github_release_link: String,
    pub service_name: String,
    program_name: String,
    startup_args: Vec<String>,
    unzip_location: String,
}

impl SystemDService {
    pub fn new(
        git_repo: String,
        service_name: String,
        program_name: String,
        startup_args: Vec<String>,
        unzip_location: Option<String>,
    ) -> Self {
        Self {
            github_release_link: git_repo,
            service_name,
            program_name,
            startup_args,
            unzip_location: unzip_location.unwrap_or("/usr/local/home_automation".to_owned()),
        }
    }

    pub fn set_args(&mut self, args: Vec<String>) {
        self.startup_args = args;
    }

    pub fn set_install_location(&mut self, new_location: &str) {
        self.unzip_location = new_location.to_owned();
    }

    pub async fn install_unit(&self) -> Result<()> {
        if !self.check_program_exists()? {
            self.download_release()?;
            self.unzip_file()?;
        }

        if !self.check_unit_file_exists()? {
            // This creates both the string and writes it to drive.
            self.create_unit_file()?;
            self.load_unit_file_from_disk().await?;
        }

        if self.check_unit_status().await? == "disabled" {
            self.enable_unit().await?;
        }

        self.start_unit().await
    }

    pub async fn uninstall_unit(&self) -> Result<()> {
        let connection = Connection::system().await?;
        let proxy = ManagerProxy::new(&connection).await?;
        proxy.stop_unit(&format!("{}.service", self.service_name), "fail").await?;
        proxy.disable_unit_files(&[&format!("{}.service", self.service_name)], false).await?;

        fs::remove_file(&format!("/etc/systemd/system/{}.service", self.service_name))?;
        proxy.reload().await?;
        // TODO: Also remove install files.
        Ok(())
    }

    pub async fn remove_installed_files(&self) -> Result<()> {
        let install_loc = Path::new(&self.unzip_location).join(&self.program_name);
        fs::remove_file(install_loc)?;
        
        match self.service_name.as_ref() {
            "substore" => {
                let files = fs::read_dir(&self.unzip_location)?;
                if files.count() == 0 {
                    fs::remove_dir(&self.unzip_location)?;
                }
            },
            _ => {},
        }

        Ok(())
    }

    fn create_unit_file(&self) -> Result<()> {
        let service_file_string = self.create_unit_file_string();
        fs::write(
            format!("/etc/systemd/system/{}.service", self.service_name),
            service_file_string?,
        )?;

        Ok(())
    }

    fn create_unit_file_string(&self) -> Result<String> {
        let program_full_path = match Path::new(&self.unzip_location)
            .canonicalize() {
                Ok(s) => {
                    s.join(&self.program_name).to_string_lossy().to_string()
                },
                Err(e) => {
                    fs::create_dir_all(&self.unzip_location)?;
                    if let Ok(new_path) = Path::new(&self.unzip_location)
                        .canonicalize() {
                            new_path.join(&self.program_name).to_string_lossy().to_string()
                    } else {
                        return Err(e.into())
                    }
                },
            };

        Ok(format!(
"[Unit]
Description=Part of the data collection package. This is the {} service. 
After=network.target

[Service]
User=root
ExecStart={} {}
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
",
            self.service_name,
            program_full_path,
            self.startup_args.join(" ")
        ))
    }

    fn check_program_exists(&self) -> Result<bool> {
        let exists = fs::exists(
            Path::new(&self.unzip_location)
                .join(&self.program_name)
        )?;

        Ok(exists)
    }

    fn check_unit_file_exists(&self) -> Result<bool> {
        let exists = fs::exists(format!("/etc/systemd/system/{}.service", self.service_name))?;
        Ok(exists)
    }


    fn download_release(&self) -> Result<()> {
        let res = reqwest::blocking::get(&self.github_release_link)?;
        let body = res.bytes()?;
        fs::write(&format!("./{}.zip", &self.service_name), body)?;

        Ok(())
    }

    fn unzip_file(&self) -> Result<String> {
        let archive_name = format!("./{}.zip", &self.service_name);
        let _ = fs::create_dir(&self.unzip_location);

        let res = std::process::Command::new("unzip")
            .args(vec!["-o", &archive_name, "-d", &self.unzip_location])
            .output()?;

        fs::remove_file(&archive_name)?;

        let stdout = String::from_utf8(res.stdout)?;
        let stderr = String::from_utf8(res.stderr)?;

        fs::set_permissions(Path::new(&self.unzip_location)
            .join(&self.program_name), Permissions::from_mode(0o775))?;

        Ok(format!("{}, {}", stdout, stderr))
    }

    async fn _check_unit_registered(&self) -> Result<bool> {
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

    /// Returns either Ok("enabled") or Ok("diabled") if the unit exists.
    pub async fn check_unit_status(&self) -> Result<String> {
        let connection = Connection::system().await?;

        let proxy = ManagerProxy::new(&connection).await?;

        let status = proxy.get_unit_file_state(&format!("{}.service", &self.service_name)).await?;

        Ok(status)
    }

    async fn load_unit_file_from_disk(&self) -> Result<()> {
        let connection = Connection::system().await?;

        let proxy = ManagerProxy::new(&connection).await?;

        proxy.load_unit(&format!("{}.service", &self.service_name)).await?;

        Ok(())
    }

    pub async fn enable_unit(&self) -> Result<()> {
        let connection = Connection::system().await?;

        let proxy = ManagerProxy::new(&connection).await?;

        let service_formatted_name = format!("{}.service", &self.service_name);

        proxy.enable_unit_files(
            &[&service_formatted_name],
            false,
            false
        ).await?;

        proxy.start_unit(&service_formatted_name, "fail").await?;
        Ok(())
    }

    pub async fn disable_unit(&self) -> Result<()> {
        let connection = Connection::system().await?;

        let proxy = ManagerProxy::new(&connection).await?;

        let service_formatted_name = format!("{}.service", &self.service_name);

        proxy.disable_unit_files(
            &[&service_formatted_name],
            false
        ).await?;

        proxy.stop_unit(&service_formatted_name, "fail").await?;
        Ok(())
    }

    async fn start_unit(&self) -> Result<()> {
        let connection = Connection::system().await?;

        let proxy = ManagerProxy::new(&connection).await?;

        proxy.start_unit(&format!("{}.service", &self.service_name), "fail").await?;

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::process;

    use super::*;

    #[test]
    fn should_fully_create_and_enable_unit() {
        fs::create_dir_all("/usr/local/home_automation/data").unwrap();
        let res: Result<()> = smol::block_on(async {
            let service = SystemDService::new(
                "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".to_owned(),
                "substore".to_owned(),
                "sub_store".to_owned(),
                vec!["--db-path".to_owned(), "/usr/local/home_automation/data/data.db".to_owned()],
                Some("/usr/local/home_automation".to_owned()),
            );

            let res = service.install_unit().await;

            assert!(res.is_ok(), "Should be able to create unit from start to finish.");

            // Cleanup
            // Not cleaning up download because we can skip downloading if we run the tests again.
            let connection = Connection::system().await?;
            let proxy = ManagerProxy::new(&connection).await?;
            proxy.stop_unit(&format!("{}.service", service.service_name), "fail").await?;
            proxy.disable_unit_files(&[&format!("{}.service", service.service_name)], false).await?;

            fs::remove_file(&format!("/etc/systemd/system/{}.service", service.service_name))?;
            proxy.reload().await?;

            Ok(())
        });

        if let Err(e) = &res {
            assert_eq!(e.to_string(), "".to_owned());
        }
    }

    #[test]
    fn should_start_unit() {
        let res: Result<()> = smol::block_on(async {
            let service = SystemDService::new(
                "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".to_owned(),
                "cron".to_owned(),
                "sub_store".to_owned(),
                vec![],
                Some("./temp".to_owned()),
            );

            service.start_unit().await?;

            Ok(())
        });

        assert!(res.is_ok(), "Should be able to start unit file.");

    }
    #[test]
    fn should_enable_unit() {
        let res: Result<()> = smol::block_on(async {
            let service = SystemDService::new(
                "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".to_owned(),
                "cron".to_owned(),
                "sub_store".to_owned(),
                vec![],
                Some("./temp".to_owned()),
            );

            service.enable_unit().await
        });

        assert!(res.is_ok(), "Should be able to enable unit file.");
    }

    #[test]
    fn should_check_unit_file_enabled() {

        let res: Result<String> = smol::block_on(async {
            let service = SystemDService::new(
                "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".to_owned(),
                "cron".to_owned(),
                "sub_store".to_owned(),
                vec![],
                Some("./temp".to_owned()),
            );

            let status = service.check_unit_status().await?;
            
            Ok(status)
        });

        if let Err(e) = &res {
            assert_eq!(e.to_string(), "".to_owned());
        }

        assert!(res.is_ok(), "Should pass if unit is enabled.");
        assert_eq!(res.unwrap(), "enabled");
    }

    #[test]
    fn should_check_unit_registered() {
        let res: Result<bool> = smol::block_on(async {
            let service = SystemDService::new(
                "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".to_owned(),
                "cron".to_owned(),
                "sub_store".to_owned(),
                vec![],
                Some("./temp".to_owned()),
            );

            let res = service._check_unit_registered().await?;

            Ok(res)
        });

        assert!(res.is_ok());
        assert!(res.expect("should be able to check unit file"));

    }

    // Ignoring this test for now to ensure we don't keep downloading the file.
    #[ignore]
    #[test]
    fn should_download_zip_file() {
        let service = SystemDService::new(
            "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".to_owned(),
            "substore".to_owned(),
            "sub_store".to_owned(),
            vec![],
            Some("./temp".to_owned()),
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
            "https://github.com/GerhardusC/SubStore/releases/latest/download/release.zip".to_owned(),
            "substore".to_owned(),
            "sub_store".to_owned(),
            vec![],
            Some("./temp".to_owned()),
        );

        // - mock setup -
        let dummy_file_name = "./sub_store";
        let dummy_zipped_name = "./substore.zip";

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
            .unzip_file()
            .expect("Should be able to unzip file");

        // ASSERT
        assert!(result.contains("extracting"));
        assert!(
            !Path::new(dummy_zipped_name)
                .try_exists()
                .expect("Should be able to call exists on file"),
            "Failed to extract file."
        );
        assert!(
            Path::new(dummy_file_name)
                .try_exists()
                .expect("Should be able to call exists on file"),
            "Extracted file does not exist."
        );

        // CLEANUP
        let _ = fs::remove_dir_all(service.unzip_location);
        let _ = fs::remove_file(dummy_file_name);
    }

    #[test]
    fn should_create_unit_file_string() {
        let service = SystemDService::new(
            "".to_owned(),
            "service_name".to_owned(),
            "program_name".to_owned(),
            vec!["-a".to_owned()],
            Some("./temp".to_owned()),
        );
        let expected_string = format!("[Unit]
Description=Part of the data collection package. This is the service_name service. 
After=network.target

[Service]
User=root
ExecStart={}/program_name -a
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
", Path::new("./").canonicalize().unwrap().join("temp").to_string_lossy().to_string());
        assert_eq!(service.create_unit_file_string().unwrap_or_else(|e| {
            println!("{:?}", e);
            "".to_owned()
        }), expected_string);
    }
}
