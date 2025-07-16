use crate::git::GitRepository;
use crate::models::GitCommit;
use anyhow::Result;
use std::collections::VecDeque;

pub struct CommitStream {
    commits: VecDeque<GitCommit>,
    repo: GitRepository,
    limit: usize,
    loaded: usize,
    batch_size: usize,
    is_complete: bool,
    next_skip: usize,
}

impl CommitStream {
    pub fn new(repo: GitRepository, limit: Option<usize>) -> Result<Self> {
        let limit = limit.unwrap_or(10000);

        Ok(Self {
            commits: VecDeque::new(),
            repo,
            limit,
            loaded: 0,
            batch_size: 50,
            is_complete: false,
            next_skip: 0,
        })
    }

    pub fn try_next(&mut self) -> Option<Result<GitCommit>> {
        // If we have commits in the queue, return one
        if let Some(commit) = self.commits.pop_front() {
            return Some(Ok(commit));
        }

        // If we're complete, return None
        if self.is_complete {
            return None;
        }

        // Try to load more commits
        if self.load_batch().is_err() {
            return None;
        }

        // Try to get a commit from the newly loaded batch
        self.commits.pop_front().map(Ok)
    }

    fn load_batch(&mut self) -> Result<()> {
        if self.is_complete {
            return Ok(());
        }

        tracing::debug!("Loading batch of commits, loaded so far: {}", self.loaded);
        // Create a fresh revwalk each time to avoid lifetime issues
        let mut revwalk = self.repo.repo().revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.push_head()?;

        // Skip commits we've already processed
        let mut skipped = 0;
        while skipped < self.next_skip {
            if revwalk.next().is_none() {
                self.is_complete = true;
                return Ok(());
            }
            skipped += 1;
        }

        let mut batch_loaded = 0;

        for oid in revwalk {
            if self.loaded >= self.limit || batch_loaded >= self.batch_size {
                break;
            }

            match oid {
                Ok(oid) => match self.repo.repo().find_commit(oid) {
                    Ok(commit) => match GitCommit::new(&commit) {
                        Ok(git_commit) => {
                            tracing::debug!(
                                "Loaded commit: {} - {}",
                                git_commit.id,
                                git_commit.message.lines().next().unwrap_or("")
                            );
                            self.commits.push_back(git_commit);
                            batch_loaded += 1;
                            self.loaded += 1;
                            self.next_skip += 1;
                        }
                        Err(e) => {
                            tracing::warn!("Error creating GitCommit: {}", e);
                            self.next_skip += 1;
                        }
                    },
                    Err(e) => {
                        tracing::warn!("Error finding commit: {}", e);
                        self.next_skip += 1;
                    }
                },
                Err(e) => {
                    tracing::warn!("Error in revwalk: {}", e);
                    self.next_skip += 1;
                }
            }
        }

        // If we loaded fewer commits than the batch size, we're done
        if batch_loaded < self.batch_size || self.loaded >= self.limit {
            self.is_complete = true;
        }

        tracing::debug!(
            "Batch complete: loaded {} commits in this batch, total loaded: {}, is_complete: {}",
            batch_loaded,
            self.loaded,
            self.is_complete
        );
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.is_complete
    }

    pub fn loaded_count(&self) -> usize {
        self.loaded
    }
}

impl std::fmt::Debug for CommitStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommitStream")
            .field("commits_count", &self.commits.len())
            .field("limit", &self.limit)
            .field("loaded", &self.loaded)
            .field("batch_size", &self.batch_size)
            .field("is_complete", &self.is_complete)
            .field("next_skip", &self.next_skip)
            .finish()
    }
}

pub struct CommitBatch {
    pub commits: Vec<GitCommit>,
    pub has_more: bool,
    pub total_processed: usize,
}

impl GitRepository {
    pub fn get_commits_streaming(&self, limit: Option<usize>) -> Result<CommitStream> {
        // Clone the repository for use in the stream
        let repo_path = self.repo().path().to_path_buf();
        let repo = GitRepository::discover(&repo_path)?;

        CommitStream::new(repo, limit)
    }
}
