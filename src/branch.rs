use git2::{BranchType, Oid, Repository};

#[derive(Debug)]
pub struct Branch {
    pub name: String,
    pub commit: String,
    pub gone: bool,
    pub current: bool,
}

pub trait BranchList {
    fn get_branches(&self) -> Vec<Branch>;
}

impl BranchList for Repository {
    fn get_branches(&self) -> Vec<Branch> {
        let branches = self.branches(None).unwrap();
        let mut output = Vec::new();
        for branch in branches
            .filter_map(|b| b.ok())
            .filter(|(_, b_type)| *b_type == BranchType::Local)
        {
            let (branch, _branch_type) = branch;
            let current = branch.is_head();
            let commit = branch.get().peel_to_commit();
            let commit: Oid = match commit {
                Ok(c) => c.id(),
                Err(e) => {
                    println!("Error getting commit: {:?}", e);
                    continue;
                }
            };
            let upstream = branch.upstream();
            let gone = match upstream {
                Ok(_) => false,
                Err(e) => matches!(e.class(), git2::ErrorClass::Reference),
            };
            let name = branch.name();
            if name.is_err() {
                println!("Error getting branch name: {:?}", name);
                continue;
            }
            let name = name.unwrap();
            if name.is_none() {
                continue;
            }
            let name = name.unwrap();

            let branch = Branch {
                name: name.to_string(),
                commit: commit.to_string().as_str()[0..8].to_string(),
                gone,
                current,
            };
            output.push(branch);
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockBranchList;

    impl BranchList for MockBranchList {
        fn get_branches(&self) -> Vec<Branch> {
            vec![
                Branch {
                    name: "master".to_string(),
                    commit: "todo".to_string(),
                    gone: false,
                    current: false,
                },
                Branch {
                    name: "feature/1".to_string(),
                    commit: "todo".to_string(),
                    gone: false,
                    current: false,
                },
                Branch {
                    name: "feature/2".to_string(),
                    commit: "todo".to_string(),
                    gone: false,
                    current: false,
                },
                Branch {
                    name: "feature/3".to_string(),
                    commit: "todo".to_string(),
                    gone: false,
                    current: false,
                },
                Branch {
                    name: "feature/4".to_string(),
                    commit: "todo".to_string(),
                    gone: false,
                    current: false,
                },
                Branch {
                    name: "feature/5".to_string(),
                    commit: "todo".to_string(),
                    gone: true,
                    current: false,
                },
                Branch {
                    name: "feature/6".to_string(),
                    commit: "todo".to_string(),
                    gone: true,
                    current: false,
                },
                Branch {
                    name: "feature/7".to_string(),
                    commit: "todo".to_string(),
                    gone: true,
                    current: false,
                },
                Branch {
                    name: "feature/8".to_string(),
                    commit: "todo".to_string(),
                    gone: true,
                    current: false,
                },
            ]
        }
    }

    #[test]
    fn test_get_branches() {
        let branches = &MockBranchList.get_branches();
        assert_eq!(branches.len(), 9);
        let gone_branch_count = branches.iter().filter(|b| b.gone).count();
        assert_eq!(gone_branch_count, 4);
    }

    #[cfg(feature = "extended_tests")]
    #[test]
    fn repository_branch_output() {
        let repo = Repository::open("../test_branch").expect("Unable to open repository");
        let branches = repo.get_branches();
        assert_eq!(branches.len(), 6);
        let gone_branch_count = branches.iter().filter(|b| b.gone).count();
        assert_eq!(gone_branch_count, 2);
    }

    #[cfg(feature = "extended_tests")]
    #[test]
    fn can_read_main_branch() {
        let repo = Repository::open("../test_branch").expect("Unable to open repository");
        let branches = repo.get_branches();
        let main_branch = branches.iter().find(|b| b.current);
        assert!(main_branch.is_some());
        assert_eq!(main_branch.unwrap().name.as_str(), "main");
    }

    #[cfg(feature = "extended_tests")]
    #[test]
    fn can_read_commit() {
        let repo = Repository::open("../test_branch").expect("Unable to open repository");
        let branches = repo.get_branches();
        let main_branch = branches.iter().find(|b| b.current);
        assert!(main_branch.is_some());
        assert_eq!(main_branch.unwrap().commit.as_str(), "8dcade56");
    }
}
