use std::path::PathBuf;

use derive_more::Constructor;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    domain::{
        models::activity::ActivityId,
        ports::{GetRawDataError, RawContent, RawDataRepository, SaveRawDataError},
    },
    inbound::parser::SupportedExtension,
};

#[derive(Debug, Clone, Constructor)]
pub struct FilesystemRawDataRepository {
    base_path: PathBuf,
}

impl RawDataRepository for FilesystemRawDataRepository {
    async fn save_raw_data(
        &self,
        activity_id: &ActivityId,
        content: RawContent,
    ) -> Result<(), SaveRawDataError> {
        let mut file = match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.target_path(activity_id, content.extension()))
            .await
        {
            Ok(file) => file,
            Err(err) => {
                tracing::warn!(
                    "Error while trying to save raw data for activity {}",
                    activity_id
                );
                tracing::warn!("{}", err);
                if err.kind() == std::io::ErrorKind::AlreadyExists {
                    return Err(SaveRawDataError::ActivityRawDataExist(activity_id.clone()));
                }
                return Err(SaveRawDataError::Unknown);
            }
        };
        file.write_all(&content.raw_content())
            .await
            .map_err(|err| {
                tracing::warn!(
                    "Error while trying to save raw data for activity {}",
                    activity_id
                );
                tracing::warn!("{}", err);
                SaveRawDataError::Unknown
            })?;
        let _ = file.flush().await;

        Ok(())
    }

    async fn get_raw_data(&self, activity_id: &ActivityId) -> Result<Vec<u8>, GetRawDataError> {
        let mut file = match tokio::fs::OpenOptions::new()
            .read(true)
            .write(false)
            .create(false)
            .open(self.target_path(activity_id, SupportedExtension::FIT.suffix()))
            .await
        {
            Ok(file) => file,
            Err(err) => {
                tracing::warn!(
                    "Error while trying to save raw data for activity {}",
                    activity_id
                );
                tracing::warn!("{}", err);
                if err.kind() == std::io::ErrorKind::NotFound {
                    return Err(GetRawDataError::NoRawDataFound(activity_id.clone()));
                }
                return Err(GetRawDataError::Unknown);
            }
        };
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).await.map_err(|err| {
            tracing::warn!(
                "Error while trying to read raw data for activity {}",
                activity_id
            );
            tracing::warn!("{}", err);
            GetRawDataError::Unknown
        })?;
        Ok(buffer)
    }
}

impl FilesystemRawDataRepository {
    fn target_path(&self, activity_id: &ActivityId, extension: &str) -> PathBuf {
        let mut path = self.base_path.clone();
        path.push(format!("{}.{}", activity_id, extension));
        path.to_path_buf()
    }
}

#[cfg(test)]
mod test_filesystem_raw_data_repository {

    use super::*;

    #[tokio::test]
    async fn test_store_raw_data() {
        let tmp_dir = tempfile::tempdir().expect("Unable to create temporary directory");
        let repository = FilesystemRawDataRepository::new(tmp_dir.path().to_path_buf());

        let activity = ActivityId::new();
        let raw_content = RawContent::new("fit".to_string(), vec![0, 1, 2, 3]);

        repository
            .save_raw_data(&activity, raw_content)
            .await
            .expect("Should have return OK");
    }

    #[tokio::test]
    async fn test_store_raw_data_already_exist() {
        let tmp_dir = tempfile::tempdir().expect("Unable to create temporary directory");
        let repository = FilesystemRawDataRepository::new(tmp_dir.path().to_path_buf());

        let activity = ActivityId::new();
        let raw_content = RawContent::new("fit".to_string(), vec![0, 1, 2, 3]);

        // Create the target activity file
        let mut path = tmp_dir.path().to_path_buf();
        path.push(format!("{}.fit", activity));
        tokio::fs::File::create(path).await.unwrap();

        let res = repository.save_raw_data(&activity, raw_content).await;

        match res {
            Err(SaveRawDataError::ActivityRawDataExist(id)) => assert_eq!(activity, id),
            _ => unreachable!("Should have return an Err"),
        }
    }

    #[tokio::test]
    async fn test_get_raw_data() {
        let tmp_dir = tempfile::tempdir().expect("Unable to create temporary directory");
        let repository = FilesystemRawDataRepository::new(tmp_dir.path().to_path_buf());

        let activity = ActivityId::new();
        let raw_content = RawContent::new("fit".to_string(), vec![0, 1, 2, 3]);

        repository
            .save_raw_data(&activity, raw_content)
            .await
            .expect("Should have return OK");

        let res = repository
            .get_raw_data(&activity)
            .await
            .expect("Should have returned OK");
        assert_eq!(res, vec![0, 1, 2, 3]);
    }

    #[tokio::test]
    async fn test_get_raw_data_does_not_exist() {
        let tmp_dir = tempfile::tempdir().expect("Unable to create temporary directory");
        let repository = FilesystemRawDataRepository::new(tmp_dir.path().to_path_buf());

        let activity = ActivityId::new();

        let res = repository.get_raw_data(&activity).await;

        match res {
            Err(GetRawDataError::NoRawDataFound(id)) => assert_eq!(activity, id),
            _ => unreachable!("Should have return an Err"),
        }
    }
}
