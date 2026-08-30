use std::collections::BTreeSet;

use super::super::actual::*;
use super::support::{replace_field, sample_actual, sample_payload};

const FROZEN_SELECTOR_NAMES: [&str; 36] = [
    "raw-state-take-rejected",
    "begin-connection-close-rejected",
    "callback-admission-rejected",
    "callback-wrapper-before",
    "shm-view-unmap-before",
    "shm-view-unmap-native-uncertain",
    "shm-view-unmap-after-known",
    "shm-view-unmap-after-uncertain",
    "shm-mapping-close-before",
    "shm-mapping-close-native-uncertain",
    "shm-mapping-close-after-known",
    "shm-mapping-close-after-uncertain",
    "shm-dms-release-before",
    "shm-dms-release-native-uncertain",
    "shm-dms-release-after-known",
    "shm-dms-release-after-uncertain",
    "shm-file-close-before",
    "shm-file-close-native-retryable",
    "shm-file-close-native-uncertain",
    "shm-file-close-after-known",
    "shm-file-close-after-uncertain",
    "shm-detach-before",
    "shm-detach-after-known",
    "shm-detach-after-uncertain",
    "main-lock-release-before",
    "main-lock-release-native-uncertain-shared",
    "main-lock-release-native-uncertain-reserved",
    "main-lock-release-after-known",
    "main-file-close-before",
    "main-file-close-native-retryable",
    "main-file-close-native-uncertain",
    "main-file-close-after-known",
    "physical-success",
    "registry-wal-main-close-before",
    "registry-wal-main-close-native-uncertain",
    "registry-wal-main-close-after-known",
];

#[test]
fn joint_close_selectors_are_an_exact_unique_name_bijection() {
    let actual_names = JointCloseSelector::ALL.map(JointCloseSelector::report_name);
    assert_eq!(actual_names, FROZEN_SELECTOR_NAMES);
    assert_eq!(actual_names.into_iter().collect::<BTreeSet<_>>().len(), 36);

    for (selector, name) in JointCloseSelector::ALL
        .into_iter()
        .zip(FROZEN_SELECTOR_NAMES)
    {
        assert_eq!(JointCloseSelector::from_report_name(name), Some(selector));
    }
    assert_eq!(
        JointCloseSelector::from_report_name("physical_success"),
        None
    );
    assert_eq!(
        JointCloseSelector::from_report_name("PhysicalSuccess"),
        None
    );
}

#[test]
fn joint_close_a2b2jc1_round_trips_all_selectors_with_eighty_three_fields() {
    for selector in JointCloseSelector::ALL {
        let actual = sample_actual(selector);
        let payload = actual.to_report_payload();
        assert!(payload.is_ascii());
        assert_eq!(payload.split(',').count(), 85);
        assert_eq!(JointCloseActual::from_report_payload(&payload), Ok(actual));
    }
}

#[test]
fn joint_close_codec_rejects_unknown_enums_and_noncanonical_booleans() {
    let payload = sample_payload();
    for (index, value) in [(9, "9"), (10, "9"), (11, "99"), (42, "9")] {
        assert!(
            JointCloseActual::from_report_payload(&replace_field(&payload, index, value,)).is_err()
        );
    }

    for index in [
        24usize, 25, 26, 30, 39, 43, 44, 45, 46, 47, 49, 51, 52, 53, 54,
    ] {
        assert!(
            JointCloseActual::from_report_payload(&replace_field(&payload, index, "2",)).is_err()
        );
    }
}

#[test]
fn joint_close_codec_rejects_missing_extra_and_overflow_fields() {
    let payload = sample_payload();
    let mut fields: Vec<_> = payload.split(',').collect();
    fields.pop();
    assert!(JointCloseActual::from_report_payload(&fields.join(",")).is_err());

    let mut fields: Vec<_> = payload.split(',').collect();
    fields.push("0");
    assert!(JointCloseActual::from_report_payload(&fields.join(",")).is_err());

    for (index, value) in [(6, "256"), (22, "4294967296"), (16, "18446744073709551616")] {
        assert!(
            JointCloseActual::from_report_payload(&replace_field(&payload, index, value,)).is_err()
        );
    }
}

#[test]
fn joint_close_codec_rejects_alias_ordering_and_noncanonical_numbers() {
    let payload = sample_payload();
    assert!(
        JointCloseActual::from_report_payload(&payload.replacen("a2b2jc1", "a2b2jc2", 1)).is_err()
    );
    for alias in ["physical_success", "unknown-joint-close-selector"] {
        assert!(JointCloseActual::from_report_payload(&replace_field(&payload, 1, alias)).is_err());
    }

    let mut reordered: Vec<_> = payload.split(',').collect();
    reordered.swap(0, 1);
    assert!(JointCloseActual::from_report_payload(&reordered.join(",")).is_err());

    for value in ["037", "+37", " 37", "37 "] {
        assert!(
            JointCloseActual::from_report_payload(&replace_field(&payload, 16, value,)).is_err()
        );
    }
    assert!(JointCloseActual::from_report_payload(&replace_field(&payload, 2, "00")).is_err());
}

#[test]
fn joint_close_codec_enforces_ascii_and_the_1024_byte_bound() {
    let payload = sample_payload();
    assert!(payload.len() < 1_024);
    assert!(JointCloseActual::from_report_payload(&payload).is_ok());

    let oversized = format!("{payload}{}", "0".repeat(1_025 - payload.len()));
    assert_eq!(oversized.len(), 1_025);
    assert!(JointCloseActual::from_report_payload(&oversized).is_err());

    let non_ascii = format!("{payload}界");
    assert!(non_ascii.len() < 1_024);
    assert!(JointCloseActual::from_report_payload(&non_ascii).is_err());
}
