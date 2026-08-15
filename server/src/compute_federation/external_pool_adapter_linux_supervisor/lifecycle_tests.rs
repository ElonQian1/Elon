use std::{cell::RefCell, rc::Rc};

use anyhow::{bail, Result};

use super::cleanup_resources;

#[test]
fn cleanup_attempts_both_resources_when_cgroup_removal_fails() {
    let attempts = Rc::new(RefCell::new(Vec::new()));
    let cgroup_attempts = Rc::clone(&attempts);
    let scratch_attempts = Rc::clone(&attempts);

    let error = cleanup_resources(
        move || {
            cgroup_attempts.borrow_mut().push("cgroup");
            bail!("injected cgroup removal failure")
        },
        move || {
            scratch_attempts.borrow_mut().push("scratch");
            Ok(())
        },
    )
    .expect_err("cgroup cleanup failure must be returned");

    assert_eq!(&*attempts.borrow(), &["cgroup", "scratch"]);
    assert_eq!(
        error.to_string(),
        "supervisor cgroup cleanup failed after reap"
    );
}

#[test]
fn cleanup_attempts_both_resources_when_scratch_removal_fails() {
    let attempts = Rc::new(RefCell::new(Vec::new()));
    let cgroup_attempts = Rc::clone(&attempts);
    let scratch_attempts = Rc::clone(&attempts);

    let error = cleanup_resources(
        move || {
            cgroup_attempts.borrow_mut().push("cgroup");
            Ok(())
        },
        move || {
            scratch_attempts.borrow_mut().push("scratch");
            bail!("injected scratch removal failure")
        },
    )
    .expect_err("scratch cleanup failure must be returned");

    assert_eq!(&*attempts.borrow(), &["cgroup", "scratch"]);
    assert_eq!(
        error.to_string(),
        "supervisor scratch cleanup failed after reap"
    );
}

#[test]
fn cleanup_reports_combined_failure_after_attempting_both_resources() {
    let attempts = Rc::new(RefCell::new(Vec::new()));
    let cgroup_attempts = Rc::clone(&attempts);
    let scratch_attempts = Rc::clone(&attempts);

    let error = cleanup_resources(
        move || {
            cgroup_attempts.borrow_mut().push("cgroup");
            bail!("injected cgroup removal failure")
        },
        move || {
            scratch_attempts.borrow_mut().push("scratch");
            bail!("injected scratch removal failure")
        },
    )
    .expect_err("combined cleanup failure must be returned");

    assert_eq!(&*attempts.borrow(), &["cgroup", "scratch"]);
    assert_eq!(
        error.to_string(),
        "supervisor cgroup and scratch cleanup failed after reap"
    );
}

#[test]
fn cleanup_succeeds_only_after_attempting_both_resources() -> Result<()> {
    let attempts = Rc::new(RefCell::new(Vec::new()));
    let cgroup_attempts = Rc::clone(&attempts);
    let scratch_attempts = Rc::clone(&attempts);

    cleanup_resources(
        move || {
            cgroup_attempts.borrow_mut().push("cgroup");
            Ok(())
        },
        move || {
            scratch_attempts.borrow_mut().push("scratch");
            Ok(())
        },
    )?;

    assert_eq!(&*attempts.borrow(), &["cgroup", "scratch"]);
    Ok(())
}
