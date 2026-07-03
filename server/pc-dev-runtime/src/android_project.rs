use crate::project_scaffold::ProjectScaffoldRequest;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

const GRADLE_WRAPPER_JAR: &[u8] =
    include_bytes!("../../../android/gradle/wrapper/gradle-wrapper.jar");
const GRADLEW_BAT: &str = include_str!("../../../android/gradlew.bat");

pub(crate) fn template_is_android(template: &str) -> bool {
    matches!(
        template.trim().to_ascii_lowercase().as_str(),
        "android" | "apk" | "android_kotlin" | "android_compose"
    )
}

pub(crate) fn ensure_android_project_files(
    repo: &Path,
    req: &ProjectScaffoldRequest<'_>,
) -> io::Result<()> {
    let package_name = package_name(req.project_id);
    let java_dir = package_name.split('.').fold(
        repo.join("app").join("src").join("main").join("java"),
        |dir, part| dir.join(part),
    );

    ensure_file(repo.join("settings.gradle"), || {
        settings_gradle(req.name.trim())
    })?;
    ensure_file(repo.join("build.gradle"), root_build_gradle)?;
    ensure_file(repo.join("gradle.properties"), gradle_properties)?;
    ensure_file(repo.join("gradlew.bat"), || Ok(GRADLEW_BAT.to_string()))?;
    ensure_file(repo.join("gradlew"), gradlew_shell)?;
    ensure_file_bytes(
        repo.join("gradle")
            .join("wrapper")
            .join("gradle-wrapper.jar"),
        GRADLE_WRAPPER_JAR,
    )?;
    ensure_file(
        repo.join("gradle")
            .join("wrapper")
            .join("gradle-wrapper.properties"),
        gradle_wrapper_properties,
    )?;
    ensure_file(repo.join("app").join("build.gradle"), || {
        app_build_gradle(&package_name)
    })?;
    ensure_file(
        repo.join("app")
            .join("src")
            .join("main")
            .join("AndroidManifest.xml"),
        android_manifest,
    )?;
    ensure_file(java_dir.join("MainActivity.java"), || {
        main_activity_java(&package_name)
    })?;
    ensure_file(
        repo.join("app")
            .join("src")
            .join("main")
            .join("res")
            .join("layout")
            .join("activity_main.xml"),
        activity_main_xml,
    )?;
    ensure_file(
        repo.join("app")
            .join("src")
            .join("main")
            .join("res")
            .join("values")
            .join("strings.xml"),
        || strings_xml(req.name.trim()),
    )?;
    Ok(())
}

fn ensure_file(path: PathBuf, content: impl FnOnce() -> io::Result<String>) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content()?)
}

fn ensure_file_bytes(path: PathBuf, content: &[u8]) -> io::Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn settings_gradle(project_name: &str) -> io::Result<String> {
    let name = gradle_string(project_name, "Elon App");
    Ok(format!(
        r#"pluginManagement {{
    repositories {{
        maven {{ url 'https://maven.aliyun.com/repository/google' }}
        maven {{ url 'https://maven.aliyun.com/repository/central' }}
        maven {{ url 'https://maven.aliyun.com/repository/gradle-plugin' }}
        google()
        mavenCentral()
        gradlePluginPortal()
    }}
}}
dependencyResolutionManagement {{
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {{
        maven {{ url 'https://maven.aliyun.com/repository/google' }}
        maven {{ url 'https://maven.aliyun.com/repository/central' }}
        google()
        mavenCentral()
    }}
}}
rootProject.name = '{name}'
include ':app'
"#
    ))
}

fn root_build_gradle() -> io::Result<String> {
    Ok(r#"plugins {
    id 'com.android.application' version '8.4.0' apply false
}
"#
    .to_string())
}

fn app_build_gradle(package_name: &str) -> io::Result<String> {
    Ok(format!(
        r#"plugins {{
    id 'com.android.application'
}}

android {{
    namespace '{package_name}'
    compileSdk 34

    defaultConfig {{
        applicationId "{package_name}"
        minSdk 26
        targetSdk 34
        versionCode 1
        versionName "1.0"
    }}
}}
"#
    ))
}

fn gradle_properties() -> io::Result<String> {
    Ok(r#"android.useAndroidX=false
org.gradle.jvmargs=-Xmx1536m -Dfile.encoding=UTF-8
"#
    .to_string())
}

fn gradle_wrapper_properties() -> io::Result<String> {
    Ok(r#"distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip
networkTimeout=10000
validateDistributionUrl=false
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
"#
    .to_string())
}

fn gradlew_shell() -> io::Result<String> {
    Ok(
        r#"#!/bin/sh
APP_HOME=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
exec java -classpath "$APP_HOME/gradle/wrapper/gradle-wrapper.jar" org.gradle.wrapper.GradleWrapperMain "$@"
"#
        .to_string(),
    )
}

fn android_manifest() -> io::Result<String> {
    Ok(r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <application
        android:allowBackup="true"
        android:label="@string/app_name"
        android:theme="@style/AppTheme">
        <activity
            android:name=".MainActivity"
            android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#
    .to_string())
}

fn main_activity_java(package_name: &str) -> io::Result<String> {
    Ok(format!(
        r#"package {package_name};

import android.app.Activity;
import android.os.Bundle;

public class MainActivity extends Activity {{
    @Override
    protected void onCreate(Bundle savedInstanceState) {{
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
    }}
}}
"#
    ))
}

fn activity_main_xml() -> io::Result<String> {
    Ok(r#"<?xml version="1.0" encoding="utf-8"?>
<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
    android:layout_width="match_parent"
    android:layout_height="match_parent"
    android:gravity="center"
    android:orientation="vertical">

    <TextView
        android:id="@+id/tvHello"
        android:layout_width="wrap_content"
        android:layout_height="wrap_content"
        android:text="@string/app_name"
        android:textSize="24sp" />
</LinearLayout>
"#
    .to_string())
}

fn strings_xml(project_name: &str) -> io::Result<String> {
    let app_name = xml_escape(project_name.trim()).filter(|value| !value.is_empty());
    let app_name = app_name.as_deref().unwrap_or("Elon App");
    Ok(format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">{app_name}</string>
    <style name="AppTheme" parent="android:style/Theme.Material.Light.NoActionBar" />
</resources>
"#
    ))
}

fn package_name(project_id: &str) -> String {
    format!("com.elon.apps.{}", package_suffix(project_id))
}

fn package_suffix(value: &str) -> String {
    let suffix = value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .take(40)
        .collect::<String>();
    if suffix.is_empty() || suffix.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        format!("p{suffix}")
    } else {
        suffix
    }
}

fn gradle_string(value: &str, fallback: &str) -> String {
    let clean = value.trim().replace('\\', "\\\\").replace('\'', "\\'");
    if clean.is_empty() {
        fallback.to_string()
    } else {
        clean
    }
}

fn xml_escape(value: &str) -> Option<String> {
    let escaped = value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;");
    (!escaped.trim().is_empty()).then_some(escaped)
}

#[cfg(test)]
mod tests {
    use super::{ensure_android_project_files, package_name, template_is_android};
    use crate::project_scaffold::ProjectScaffoldRequest;
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn detects_android_templates() {
        assert!(template_is_android("android"));
        assert!(template_is_android("android_kotlin"));
        assert!(template_is_android("android_compose"));
        assert!(!template_is_android("web"));
    }

    #[test]
    fn creates_buildable_android_skeleton_without_overwrite() {
        let root = temp_dir("android_skeleton");
        let req = request();
        fs::create_dir_all(root.join("app")).unwrap();
        fs::write(root.join("app").join("build.gradle"), "custom").unwrap();

        ensure_android_project_files(&root, &req).unwrap();

        assert!(root.join("settings.gradle").exists());
        assert!(root.join("build.gradle").exists());
        assert!(root.join("gradlew.bat").exists());
        assert!(root.join("gradle/wrapper/gradle-wrapper.jar").exists());
        assert_eq!(
            fs::read_to_string(root.join("app").join("build.gradle")).unwrap(),
            "custom"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn package_name_is_java_safe() {
        assert_eq!(package_name("prj_123-ABC"), "com.elon.apps.prj123abc");
        assert_eq!(package_name("123"), "com.elon.apps.p123");
    }

    fn request() -> ProjectScaffoldRequest<'static> {
        ProjectScaffoldRequest {
            project_id: "prj_123",
            user_id: "usr_1",
            name: "Demo",
            template: "android_kotlin",
            repo_url: None,
            branch: None,
        }
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elon-pc-dev-runtime-{label}-{nanos}"))
    }
}
