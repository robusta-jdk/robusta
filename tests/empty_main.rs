use anyhow::Error;
use assert_cmd::Command;

#[test]
fn empty_main() -> Result<(), Error> {
    let mut cmd = Command::cargo_bin("robusta")?;
    
    cmd.arg("com.jkitch.robusta.test.EmptyMain")
        .assert()
        .success();
    
    Ok(())
}
