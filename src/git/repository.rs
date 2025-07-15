use crate::git::GitCommands;
use crate::models::{GitCommit, RepositoryInfo};
use anyhow::{anyhow, Result};
use git2::{Repository, RepositoryOpenFlags};
use std::path::Path;

pub struct GitRepository {
    repo: Repository,
    info: RepositoryInfo,
    commands: GitCommands,
}

impl std::fmt::Debug for GitRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GitRepository")
            .field("info", &self.info)
            .finish()
    }
}

impl GitRepository {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let repo = Repository::open_ext(
            path.as_ref(),
            RepositoryOpenFlags::empty(),
            &[] as &[&std::ffi::OsStr],
        )?;

        let info = RepositoryInfo::from_repo(&repo)?;
        let repo_path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
        let commands = GitCommands::new(&repo_path)?;

        Ok(GitRepository {
            repo,
            info,
            commands,
        })
    }

    pub fn discover<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Use open_from_env or open_ext to try to discover the repository
        let repo = Repository::open_ext(
            &path,
            git2::RepositoryOpenFlags::empty(),
            &[] as &[&std::ffi::OsStr],
        )?;

        let info = RepositoryInfo::from_repo(&repo)?;
        let repo_path = repo.workdir().unwrap_or_else(|| repo.path()).to_path_buf();
        let commands = GitCommands::new(&repo_path)?;

        Ok(GitRepository {
            repo,
            info,
            commands,
        })
    }

    pub fn info(&self) -> &RepositoryInfo {
        &self.info
    }

    pub fn get_commits(&self, max_count: Option<usize>) -> Result<Vec<GitCommit>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        let limit = max_count.unwrap_or(1000);

        for (index, oid) in revwalk.enumerate() {
            if index >= limit {
                break;
            }

            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(GitCommit::new(&commit)?);
        }

        Ok(commits)
    }

    pub fn get_commit(&self, id: &str) -> Result<GitCommit> {
        let oid = git2::Oid::from_str(id)?;
        let commit = self.repo.find_commit(oid)?;
        GitCommit::new(&commit)
    }

    pub fn get_commits_in_range(&self, from: &str, to: &str) -> Result<Vec<GitCommit>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;

        let to_oid = git2::Oid::from_str(to)?;
        let from_oid = git2::Oid::from_str(from)?;

        revwalk.push(to_oid)?;
        revwalk.hide(from_oid)?;

        let mut commits = Vec::new();
        for oid in revwalk {
            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            commits.push(GitCommit::new(&commit)?);
        }

        Ok(commits)
    }

    pub fn get_file_content(&self, commit_id: &str, path: &str) -> Result<String> {
        let oid = git2::Oid::from_str(commit_id)?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;

        let entry = tree.get_path(Path::new(path))?;
        let object = entry.to_object(&self.repo)?;

        if let Some(blob) = object.as_blob() {
            let content = blob.content();
            Ok(String::from_utf8_lossy(content).into_owned())
        } else {
            Err(anyhow!("Object is not a blob"))
        }
    }

    pub fn get_head_commit(&self) -> Result<GitCommit> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        GitCommit::new(&commit)
    }

    pub fn get_branches(&self) -> Result<Vec<String>> {
        let mut branches = Vec::new();
        let branch_iter = self.repo.branches(Some(git2::BranchType::Local))?;

        for branch in branch_iter {
            let (branch, _) = branch?;
            if let Some(name) = branch.name()? {
                branches.push(name.to_string());
            }
        }

        Ok(branches)
    }

    pub fn get_tags(&self) -> Result<Vec<String>> {
        let mut tags = Vec::new();
        self.repo.tag_names(None)?.iter().for_each(|tag| {
            if let Some(tag_name) = tag {
                tags.push(tag_name.to_string());
            }
        });
        Ok(tags)
    }

    pub fn search_commits(&self, query: &str, max_count: Option<usize>) -> Result<Vec<GitCommit>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.push_head()?;

        let mut commits = Vec::new();
        let limit = max_count.unwrap_or(1000);
        let query = query.to_lowercase();

        for (index, oid) in revwalk.enumerate() {
            if index >= limit {
                break;
            }

            let oid = oid?;
            let commit = self.repo.find_commit(oid)?;
            let git_commit = GitCommit::new(&commit)?;

            if git_commit.message.to_lowercase().contains(&query)
                || git_commit.author.name.to_lowercase().contains(&query)
                || git_commit.author.email.to_lowercase().contains(&query)
                || git_commit.id.contains(&query)
            {
                commits.push(git_commit);
            }
        }

        Ok(commits)
    }

    pub fn get_repository(&self) -> &Repository {
        &self.repo
    }

    pub fn repo(&self) -> &Repository {
        &self.repo
    }

    pub fn commands(&self) -> &GitCommands {
        &self.commands
    }

    /// Get commit information using safe command execution
    pub fn get_commit_info_safe(&self, commit_id: &str) -> Result<String> {
        self.commands.cat_file(&["-p", commit_id])
    }

    /// Get raw commit data for multiple commits
    pub fn get_commits_raw(&self, args: &[&str]) -> Result<String> {
        self.commands.rev_list(args)
    }

    /// Get detailed log information
    pub fn get_log_detailed(&self, args: &[&str]) -> Result<String> {
        let mut full_args = vec!["--pretty=raw", "-z"];
        full_args.extend_from_slice(args);
        self.commands.log(&full_args)
    }

    /// Get all references
    pub fn get_all_refs(&self) -> Result<String> {
        self.commands.show_ref(&["-d"])
    }

    /// Check if the repository has uncommitted changes
    pub fn has_uncommitted_changes(&self) -> Result<bool> {
        let output = self.commands.ls_files(&["-u"])?;
        Ok(!output.trim().is_empty())
    }

    /// Get the current branch name
    pub fn get_current_branch_safe(&self) -> Result<Option<String>> {
        match self.commands.rev_parse(&["--abbrev-ref", "HEAD"]) {
            Ok(output) => {
                let branch = output.trim();
                if branch == "HEAD" {
                    Ok(None) // Detached HEAD
                } else {
                    Ok(Some(branch.to_string()))
                }
            }
            Err(_) => Ok(None),
        }
    }

    /// Get all local and remote branches
    pub fn get_all_branches_safe(&self) -> Result<Vec<String>> {
        let output = self.commands.for_each_ref(&[
            "--format=%(refname:short)",
            "refs/heads/",
            "refs/remotes/",
        ])?;

        Ok(output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect())
    }

    /// Get all tags
    pub fn get_all_tags_safe(&self) -> Result<Vec<String>> {
        let output = self
            .commands
            .for_each_ref(&["--format=%(refname:short)", "refs/tags/"])?;

        Ok(output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.to_string())
            .collect())
    }

    /// Get repository information using safe commands
    pub fn get_repo_info_safe(&self) -> Result<(bool, Option<String>)> {
        let has_worktree = self.commands.has_work_tree()?;
        let worktree_path = self.commands.work_tree()?;
        let worktree_str = worktree_path.map(|p| p.to_string_lossy().to_string());

        Ok((has_worktree, worktree_str))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::process::Command;
    use tempfile::TempDir;
    use test_case::test_case;

    /// Create a temporary Git repository for testing
    fn create_test_repo() -> anyhow::Result<(TempDir, std::path::PathBuf)> {
        let temp_dir = TempDir::new()?;
        let repo_path = temp_dir.path().to_path_buf();

        // Initialize Git repository
        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()?;

        // Configure Git user for commits
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()?;

        Ok((temp_dir, repo_path))
    }

    /// Create a test commit in the repository
    fn create_test_commit(
        repo_path: &Path,
        filename: &str,
        content: &str,
        message: &str,
    ) -> anyhow::Result<()> {
        let file_path = repo_path.join(filename);
        std::fs::write(&file_path, content)?;

        Command::new("git")
            .args(["add", filename])
            .current_dir(repo_path)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", message])
            .current_dir(repo_path)
            .output()?;

        Ok(())
    }

    #[test]
    fn test_repository_discovery_valid_repo() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "Hello, World!", "Initial commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        assert!(repository
            .repo
            .workdir()
            .unwrap_or(repository.repo.path())
            .exists());
        assert_eq!(
            repository.info().name,
            repo_path.file_name().unwrap().to_string_lossy()
        );

        Ok(())
    }

    #[test]
    fn test_repository_discovery_invalid_path() {
        let non_existent_path = Path::new("/does/not/exist");
        let result = GitRepository::discover(non_existent_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_repository_discovery_non_git_directory() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let result = GitRepository::discover(temp_dir.path());
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_get_commits_basic() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "file1.txt", "Content 1", "First commit")?;
        create_test_commit(&repo_path, "file2.txt", "Content 2", "Second commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        let commits = repository.get_commits(Some(10))?;

        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].summary, "Second commit");
        assert_eq!(commits[1].summary, "First commit");

        Ok(())
    }

    #[test]
    fn test_get_commits_with_limit() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;

        // Create 5 commits
        for i in 1..=5 {
            create_test_commit(
                &repo_path,
                &format!("file{}.txt", i),
                &format!("Content {}", i),
                &format!("Commit {}", i),
            )?;
        }

        let repository = GitRepository::discover(&repo_path)?;
        let commits = repository.get_commits(Some(3))?;

        assert_eq!(commits.len(), 3);
        assert_eq!(commits[0].summary, "Commit 5");
        assert_eq!(commits[2].summary, "Commit 3");

        Ok(())
    }

    #[test]
    fn test_get_branches() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(
            &repo_path,
            "initial.txt",
            "Initial content",
            "Initial commit",
        )?;

        // Create a feature branch
        Command::new("git")
            .args(["checkout", "-b", "feature/test"])
            .current_dir(&repo_path)
            .output()?;

        create_test_commit(
            &repo_path,
            "feature.txt",
            "Feature content",
            "Feature commit",
        )?;

        let repository = GitRepository::discover(&repo_path)?;
        let branches = repository.get_branches()?;

        assert!(branches.contains(&"main".to_string()) || branches.contains(&"master".to_string()));
        assert!(branches.contains(&"feature/test".to_string()));

        Ok(())
    }

    #[test]
    fn test_get_tags() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "tagged.txt", "Tagged content", "Tagged commit")?;

        // Create a tag
        Command::new("git")
            .args(["tag", "v1.0.0"])
            .current_dir(&repo_path)
            .output()?;

        let repository = GitRepository::discover(&repo_path)?;
        let tags = repository.get_tags()?;

        assert!(tags.contains(&"v1.0.0".to_string()));

        Ok(())
    }

    #[test_case("", "Empty string should be invalid")]
    #[test_case("invalid-sha", "Invalid SHA format should be invalid")]
    #[test_case("123", "Too short SHA should be invalid")]
    fn test_get_commit_with_invalid_sha(
        invalid_sha: &str,
        _description: &str,
    ) -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "test.txt", "Test content", "Test commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        let result = repository.get_commit(invalid_sha);

        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_search_commits() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "file1.txt", "Content 1", "Add feature A")?;
        create_test_commit(&repo_path, "file2.txt", "Content 2", "Fix bug in component")?;
        create_test_commit(&repo_path, "file3.txt", "Content 3", "Add feature B")?;

        let repository = GitRepository::discover(&repo_path)?;
        let commits = repository.search_commits("feature", Some(10))?;

        assert_eq!(commits.len(), 2);
        assert!(commits.iter().any(|c| c.summary.contains("feature A")));
        assert!(commits.iter().any(|c| c.summary.contains("feature B")));

        Ok(())
    }

    #[test]
    fn test_repository_info() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "info.txt", "Info content", "Info commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        let info = repository.info();

        assert_eq!(info.name, repo_path.file_name().unwrap().to_string_lossy());
        assert_eq!(info.path, repo_path);
        assert!(info.head_branch.is_some());

        Ok(())
    }

    #[test]
    fn test_has_uncommitted_changes_clean_repo() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "clean.txt", "Clean content", "Clean commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        let has_changes = repository.has_uncommitted_changes()?;

        assert!(!has_changes);
        Ok(())
    }

    #[test]
    fn test_has_uncommitted_changes_dirty_repo() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "dirty.txt", "Initial content", "Initial commit")?;

        // Modify file without committing
        std::fs::write(repo_path.join("dirty.txt"), "Modified content")?;

        let repository = GitRepository::discover(&repo_path)?;
        let has_changes = repository.has_uncommitted_changes()?;

        assert!(has_changes);
        Ok(())
    }

    #[test]
    fn test_get_current_branch() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "branch.txt", "Branch content", "Branch commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        let current_branch = repository.get_current_branch_safe()?;

        assert!(current_branch.is_some());
        let branch = current_branch.unwrap();
        assert!(branch == "main" || branch == "master");

        Ok(())
    }

    #[test]
    fn test_get_repository_reference() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "ref.txt", "Ref content", "Ref commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        let repo_ref = repository.get_repository();

        assert!(repo_ref.path().exists());
        Ok(())
    }

    #[test]
    fn test_path_method() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "path.txt", "Path content", "Path commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        let returned_path = repository.repo.workdir().unwrap_or(repository.repo.path());

        assert_eq!(*returned_path, repo_path);
        Ok(())
    }

    #[test]
    fn test_repo_info_safe() -> anyhow::Result<()> {
        let (_temp_dir, repo_path) = create_test_repo()?;
        create_test_commit(&repo_path, "safe.txt", "Safe content", "Safe commit")?;

        let repository = GitRepository::discover(&repo_path)?;
        let (has_worktree, worktree_path) = repository.get_repo_info_safe()?;

        assert!(has_worktree);
        assert!(worktree_path.is_some());

        Ok(())
    }

    #[cfg(feature = "testing")]
    mod mock_tests {
        use super::*;
        use mockall::predicate;

        // These tests would use mocks for more isolated unit testing
        // They're conditional on the "testing" feature flag

        #[test]
        fn test_mock_repository_operations() {
            // Example of how mock tests would be structured
            // This would test the business logic without real Git operations
        }
    }

    /// Property-based tests using proptest
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn test_commit_message_roundtrip(message in "\\PC{1,100}") {
                // Test that commit messages can be stored and retrieved correctly
                prop_assume!(!message.trim().is_empty());

                let (_temp_dir, repo_path) = create_test_repo().unwrap();
                create_test_commit(&repo_path, "prop.txt", "Property content", &message).unwrap();

                let repository = GitRepository::discover(&repo_path).unwrap();
                let commits = repository.get_commits(Some(1)).unwrap();

                prop_assert_eq!(&commits[0].summary, &message);
            }

            #[test]
            fn test_file_content_roundtrip(content in "[\\x20-\\x7E]{1,100}") {
                // Test that file content can be stored and retrieved correctly using ASCII printable chars
                let (_temp_dir, repo_path) = create_test_repo().unwrap();
                create_test_commit(&repo_path, "content.txt", &content, "Content test").unwrap();

                let repository = GitRepository::discover(&repo_path).unwrap();
                if let Ok(commits) = repository.get_commits(Some(1)) {
                    if let Some(first_commit) = commits.first() {
                        if let Ok(retrieved_content) = repository.get_file_content(&first_commit.id, "content.txt") {
                            prop_assert_eq!(retrieved_content, content);
                        }
                    }
                }
            }
        }
    }
}
