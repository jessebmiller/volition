use uuid::Uuid;
use git2::Repository;
use anyhow::Context;

pub fn create_unique_branch_name() -> String {
    let unique_branch_name = format!("branch-{}", Uuid::new_v4());
    unique_branch_name
}

pub async fn commit_current_state(repo_path: &str, message: &str) -> Result<String, anyhow::Error> {
    let repo = Repository::open(repo_path)?;
    let mut index = repo.index()?;
    index.add_all(["."].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let oid = index.write_tree()?;
    let signature = repo.signature()?;
    let parent_commit = repo.head()?.peel_to_commit()?;
    let tree = repo.find_tree(oid)?;
    let commit_oid = repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent_commit])?;
    Ok(commit_oid.to_string())
}

pub async fn tag_solution(repo_path: &str, solution: &str, tag: &str) -> Result<(), anyhow::Error> {
    let repo = Repository::open(repo_path)?;
    let object = repo.revparse_single(solution)?;
    let signature = repo.signature()?;
    repo.tag(tag, &object, &signature, &format!("Tagging solution {}", solution), false)?;
    Ok(())
}

pub async fn checkout_solution(repo_path: &str, solution: &str) -> Result<(), anyhow::Error> {
    let repo = Repository::open(repo_path)?;
    let (object, reference) = repo.revparse_ext(solution)?;
    repo.checkout_tree(&object, None)?;
    match reference {
        Some(r) => repo.set_head(r.name().unwrap())?,
        None => repo.set_head_detached(object.id())?,
    }
    Ok(())
}

pub async fn cleanup(repo_path: &str) -> Result<(), anyhow::Error> {
    let repo = Repository::open(repo_path)?;
    repo.cleanup_state().context("Failed to cleanup repository state")?;
    Ok(())
}

pub async fn create_and_checkout_branch(repo_path: &str, branch_name: &str) -> Result<(), anyhow::Error> {
    let repo = Repository::open(repo_path)?;
    let head_commit = repo.head()?.peel_to_commit()?;
    let branch = repo.branch(branch_name, &head_commit, false)?;
    let branch_ref = branch.get().name().unwrap().to_string();
    repo.set_head(&branch_ref)?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
    Ok(())
}
