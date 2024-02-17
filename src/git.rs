use std::{path::Path, time::Duration};

use git2::{Cred, FetchOptions, PushOptions, RemoteCallbacks, Repository, ResetType, Signature};

use crate::{Result, GH_AUTHOR, GH_EMAIL, GH_PAT, GH_USERNAME};

use anyhow::{anyhow, Context};

pub struct TrunkRepo {
    inner: Repository
}

impl TrunkRepo {
    pub fn clone(save_to: &Path) -> Result<Self> {
        Ok(Self {
            inner: Repository::clone("https://github.com/vrmiguel/trunk.git", save_to)?,
        })
    }

    fn commit_to_branch(&self, message: &str, author: &str, email: &str, branch_name: &str) -> Result {
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
        let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, parents)?;

        let commit = repo.find_commit(oid)?;

        repo.branch(branch_name, &commit, false)?;

        Ok(())
    }


//        self.inner.branch(branch_name, &head, false)?;

    pub fn commit_and_push(&mut self, commit_message: &str, branch_name: &str) -> Result {
        println!("Branch name is {branch_name}");

        self.commit_to_branch(commit_message, &GH_AUTHOR, &GH_EMAIL, branch_name)?;

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
