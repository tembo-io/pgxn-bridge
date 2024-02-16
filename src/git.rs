use std::path::Path;

use git2::{Cred, PushOptions, RemoteCallbacks, Repository, Signature};

use crate::{Result, GH_AUTHOR, GH_EMAIL, GH_PAT, GH_USERNAME};

use anyhow::{anyhow, Context};

pub struct TrunkRepo {
    inner: Repository,
    current_branch: Option<String>,
}

impl TrunkRepo {
    pub fn clone(save_to: &Path) -> Result<Self> {
        Ok(Self {
            inner: Repository::clone("https://github.com/tembo-io/trunk.git", save_to)?,
            current_branch: None,
        })
    }

    pub fn create_branch(&mut self, branch_name: &str) -> Result {
        let head = self.inner.head()?.peel_to_commit()?;
        self.inner.branch(branch_name, &head, false)?;

        self.current_branch = Some(branch_name.into());

        println!("Created branch {branch_name}");

        Ok(())
    }

    fn commit(&self, message: &str, author: &str, email: &str) -> Result {
        let repo = &self.inner;

        // Create the signature for the commit
        let sig = Signature::now(author, email)?;

        // Get the current index
        let mut index = repo.index()?;

        // equivalent to `git add .`)
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;

        // Write the index to a tree
        let tree_id = index.write_tree()?;
        let tree = repo.find_tree(tree_id)?;

        // Get the HEAD as the parent for the commit
        let obj = self
            .inner
            .head()?
            .resolve()?
            .peel(git2::ObjectType::Commit)?;
        let parent_commit = obj.into_commit().map_err(|_| anyhow!("Commit not found"))?;

        let parents = &[&parent_commit];

        // Commit the changes
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, parents)?;

        Ok(())
    }

    pub fn commit_and_push(&mut self, commit_message: &str) -> Result {
        let branch_name = self
            .current_branch
            .as_deref()
            .with_context(|| "Expected a branch to be set")?;

        self.commit(commit_message, &GH_AUTHOR, &GH_EMAIL)?;

        let mut remote = self.inner.find_remote("origin")?;

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            Cred::userpass_plaintext(&GH_USERNAME, &GH_PAT)
        });

        let mut opts = PushOptions::new();
        opts.remote_callbacks(callbacks);

        remote.push(&[format!("refs/heads/{}", branch_name)], Some(&mut opts))?;

        Ok(())
    }
}
