const OFFICIAL_QUANT_PROJECT_ID: &str = "yilong-quant";

pub(super) struct PublishedReleasePresentation {
    pub(super) download_url: String,
    pub(super) identity_url: String,
}

pub(super) fn published_release_presentation(
    project_id: &str,
    public_url: &str,
    release_url: &str,
) -> PublishedReleasePresentation {
    let download_url = if project_id == OFFICIAL_QUANT_PROJECT_ID {
        crate::project_store::apk::android_download_route(public_url, project_id)
    } else {
        release_url.to_owned()
    };
    PublishedReleasePresentation {
        download_url,
        identity_url: release_url.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn official_quant_release_uses_fixed_public_route_and_actual_url_for_identity() {
        let actual_release_url =
            "https://main.example/api/projects/yilong-quant/download/latest.apk";

        let presentation = published_release_presentation(
            "yilong-quant",
            "https://main.example/",
            actual_release_url,
        );

        assert_eq!(
            presentation.download_url,
            "https://main.example/api/store/projects/yilong-quant/downloads/android"
        );
        assert_eq!(presentation.identity_url, actual_release_url);
    }

    #[test]
    fn other_projects_keep_their_release_url() {
        let actual_release_url = "https://main.example/api/projects/merchant/download/latest.apk";

        let presentation =
            published_release_presentation("merchant", "https://main.example", actual_release_url);

        assert_eq!(presentation.download_url, actual_release_url);
        assert_eq!(presentation.identity_url, actual_release_url);
    }
}
