#!/bin/bash
# 在服务器上创建 Android 项目模板
set -e
TMPL=/root/templates/android
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WRAPPER_JAR="$SCRIPT_DIR/../android/gradle/wrapper/gradle-wrapper.jar"

echo "创建目录结构..."
mkdir -p $TMPL/app/src/main/kotlin/com/template/app
mkdir -p $TMPL/app/src/main/res/layout
mkdir -p $TMPL/app/src/main/res/values
mkdir -p $TMPL/gradle/wrapper

# ── settings.gradle ────────────────────────────────────────────
cat > $TMPL/settings.gradle << 'SETTINGS'
pluginManagement {
    repositories {
        maven { url 'https://maven.aliyun.com/repository/google' }
        maven { url 'https://maven.aliyun.com/repository/central' }
        maven { url 'https://maven.aliyun.com/repository/gradle-plugin' }
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}
dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        maven { url 'https://maven.aliyun.com/repository/google' }
        maven { url 'https://maven.aliyun.com/repository/central' }
        google()
        mavenCentral()
    }
}
rootProject.name = "TemplateApp"
include ':app'
SETTINGS

# ── build.gradle (root) ────────────────────────────────────────
cat > $TMPL/build.gradle << 'BUILDROOT'
plugins {
    id 'com.android.application' version '8.4.0' apply false
    id 'org.jetbrains.kotlin.android' version '1.9.23' apply false
}
BUILDROOT

# ── app/build.gradle ───────────────────────────────────────────
cat > $TMPL/app/build.gradle << 'APPBUILD'
plugins {
    id 'com.android.application'
    id 'org.jetbrains.kotlin.android'
}

android {
    namespace 'com.template.app'
    compileSdk 34

    defaultConfig {
        applicationId "com.template.app"
        minSdk 26
        targetSdk 34
        versionCode 1
        versionName "1.0"
    }

    buildTypes {
        release {
            minifyEnabled false
        }
    }
    compileOptions {
        sourceCompatibility JavaVersion.VERSION_1_8
        targetCompatibility JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = '1.8'
    }
}

dependencies {
    implementation 'androidx.core:core-ktx:1.12.0'
    implementation 'androidx.appcompat:appcompat:1.6.1'
    implementation 'com.google.android.material:material:1.11.0'
    implementation 'androidx.constraintlayout:constraintlayout:2.1.4'
}
APPBUILD

# ── gradle.properties ──────────────────────────────────────────
cat > $TMPL/gradle.properties << 'GRADLEPROP'
android.useAndroidX=true
android.enableJetifier=true
org.gradle.jvmargs=-Xmx1024m -Dfile.encoding=UTF-8
kotlin.code.style=official
GRADLEPROP

cat > $TMPL/local.properties << 'LOCALPROPS'
sdk.dir=/root/android-sdk
LOCALPROPS

# ── gradle/wrapper/gradle-wrapper.properties ───────────────────
cat > $TMPL/gradle/wrapper/gradle-wrapper.properties << 'WRAPPERPROPS'
distributionBase=GRADLE_USER_HOME
distributionPath=wrapper/dists
distributionUrl=https\://mirrors.cloud.tencent.com/gradle/gradle-8.6-bin.zip
networkTimeout=10000
validateDistributionUrl=false
zipStoreBase=GRADLE_USER_HOME
zipStorePath=wrapper/dists
WRAPPERPROPS

# ── AndroidManifest.xml ────────────────────────────────────────
cat > $TMPL/app/src/main/AndroidManifest.xml << 'MANIFEST'
<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <application
        android:allowBackup="true"
        android:label="@string/app_name"
        android:theme="@style/Theme.AppCompat.Light.DarkActionBar">
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
MANIFEST

# ── MainActivity.kt ────────────────────────────────────────────
cat > $TMPL/app/src/main/kotlin/com/template/app/MainActivity.kt << 'MAINKT'
package com.template.app

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import android.widget.TextView

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)
    }
}
MAINKT

# ── activity_main.xml ──────────────────────────────────────────
cat > $TMPL/app/src/main/res/layout/activity_main.xml << 'LAYOUT'
<?xml version="1.0" encoding="utf-8"?>
<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
    android:layout_width="match_parent"
    android:layout_height="match_parent"
    android:gravity="center"
    android:orientation="vertical">

    <TextView
        android:id="@+id/tvHello"
        android:layout_width="wrap_content"
        android:layout_height="wrap_content"
        android:text="Hello World!"
        android:textSize="24sp" />

</LinearLayout>
LAYOUT

# ── strings.xml ────────────────────────────────────────────────
cat > $TMPL/app/src/main/res/values/strings.xml << 'STRINGS'
<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">Template App</string>
</resources>
STRINGS

# ── gradlew ────────────────────────────────────────────────────
cat > $TMPL/gradlew << 'GRADLEW'
#!/bin/sh
APP_HOME=$(dirname "$(readlink -f "$0")" 2>/dev/null || dirname "$(python3 -c 'import os,sys;print(os.path.realpath(sys.argv[1]))' "$0")")
exec java -classpath "$APP_HOME/gradle/wrapper/gradle-wrapper.jar" \
  org.gradle.wrapper.GradleWrapperMain "$@"
GRADLEW
chmod +x $TMPL/gradlew

if [ -f "$WRAPPER_JAR" ]; then
    cp "$WRAPPER_JAR" $TMPL/gradle/wrapper/gradle-wrapper.jar
else
    echo "ERROR: Gradle wrapper jar not found at $WRAPPER_JAR" >&2
    exit 1
fi

# ── .gitignore ─────────────────────────────────────────────────
cat > $TMPL/.gitignore << 'GITIGNORE'
*.iml
.gradle
/local.properties
/.idea
.DS_Store
/build
/captures
.externalNativeBuild
.cxx
local.properties
GITIGNORE

echo "模板文件创建完成，目录结构："
find $TMPL -type f | sort
