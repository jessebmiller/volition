use std::process::Command;
use std::error::Error;

const NAMESPACE: &str = "simulated_annealing";

pub async fn commit_current_state(message: &str, debug_level: DebugLevel) -> Result<String, Box<dyn Error>> {
    // Create an orphan branch for the commit
    Command::new("git")
        .args(["checkout", "--orphan", &format!("{}-branch", NAMESPACE)])
        .output()?;

    // Execute git add and commit commands
    Command::new("git")
        .args(["add", "."])
        .output()?;

    let output = Command::new("git")
        .args(["commit", "-m", message])
        .output()?;

    if debug_level >= DebugLevel::Info {
        println!("Commit message: {}", message);
    }

    // Extract commit hash from the output
    let commit_hash = String::from_utf8(output.stdout)?;
    Ok(commit_hash.trim().to_string())
}

pub async fn tag_solution(commit_hash: &str, tag_name: &str, debug_level: DebugLevel) -> Result<(), Box<dyn Error>> {
    Command::new("git")
        .args(["tag", &format!("{}-{}", NAMESPACE, tag_name), commit_hash])
        .output()?;

    if debug_level >= DebugLevel::Info {
        println!("Tagged commit {} as {}", commit_hash, tag_name);
    }

    Ok(())
}

pub async fn checkout_solution(commit_hash: &str, debug_level: DebugLevel) -> Result<(), Box<dyn Error>> {
    Command::new("git")
        .args(["checkout", commit_hash])
        .output()?;

    if debug_level >= DebugLevel::Info {
        println!("Checked out commit {}", commit_hash);
    }

    Ok(())
}

pub async fn get_diff(commit_hash1: &str, commit_hash2: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["diff", commit_hash1, commit_hash2])
        .output()?;

    let diff = String::from_utf8(output.stdout)?;
    Ok(diff)
}

pub async fn cleanup(debug_level: DebugLevel) -> Result<(), Box<dyn Error>> {
    // Delete all branches and tags in the special namespace
    Command::new("git")
        .args(["branch", "-D", &format!("{}-branch", NAMESPACE)])
        .output()?;

    Command::new("git")
        .args(["tag", "-d", &format!("{}-*", NAMESPACE)])
        .output()?;

    if debug_level >= DebugLevel::Info {
        println!("Cleaned up simulated annealing branches and tags");
    }

    Ok(())
}