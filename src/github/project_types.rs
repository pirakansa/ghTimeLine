use crate::github::project::ProjectLocator;
use crate::storage::items::StreamItemUpsert;

const PROJECT_ITEMS_FRAGMENT: &str = r#"
fragment ProjectItems on ProjectV2 {
  items(first: $first, after: $after) {
    pageInfo {
      hasNextPage
      endCursor
    }
    nodes {
      id
      type
      isArchived
      updatedAt
      content {
        ... on Issue {
          id
          number
          title
          url
          state
          createdAt
          updatedAt
          closedAt
          comments {
            totalCount
          }
          repository {
            nameWithOwner
          }
          author {
            login
            avatarUrl
          }
          labels(first: 20) {
            nodes {
              name
            }
          }
          assignees(first: 20) {
            nodes {
              login
              avatarUrl
            }
          }
        }
        ... on PullRequest {
          id
          number
          title
          url
          state
          isDraft
          merged
          mergedAt
          createdAt
          updatedAt
          closedAt
          comments {
            totalCount
          }
          repository {
            nameWithOwner
          }
          author {
            login
            avatarUrl
          }
          labels(first: 20) {
            nodes {
              name
            }
          }
          assignees(first: 20) {
            nodes {
              login
              avatarUrl
            }
          }
        }
      }
    }
  }
}
"#;

const PROJECT_BY_ID_QUERY: &str = r#"
query ProjectById($projectId: ID!, $first: Int!, $after: String) {
  node(id: $projectId) {
    ... on ProjectV2 {
      ...ProjectItems
    }
  }
}
"#;

const PROJECT_BY_ORG_QUERY: &str = r#"
query ProjectByOrg($owner: String!, $number: Int!, $first: Int!, $after: String) {
  organization(login: $owner) {
    projectV2(number: $number) {
      ...ProjectItems
    }
  }
}
"#;

const PROJECT_BY_USER_QUERY: &str = r#"
query ProjectByUser($owner: String!, $number: Int!, $first: Int!, $after: String) {
  user(login: $owner) {
    projectV2(number: $number) {
      ...ProjectItems
    }
  }
}
"#;

#[derive(serde::Serialize)]
pub(super) struct ProjectRequest<'a> {
    pub(super) query: String,
    pub(super) variables: ProjectVariables<'a>,
}

impl<'a> ProjectRequest<'a> {
    pub(super) fn new(locator: &'a ProjectLocator, first: usize, after: Option<&'a str>) -> Self {
        let (query, variables) = match locator {
            ProjectLocator::NodeId(project_id) => (
                format!("{PROJECT_BY_ID_QUERY}\n{PROJECT_ITEMS_FRAGMENT}"),
                ProjectVariables {
                    project_id: Some(project_id.as_str()),
                    owner: None,
                    number: None,
                    first,
                    after,
                },
            ),
            ProjectLocator::Organization { owner, number } => (
                format!("{PROJECT_BY_ORG_QUERY}\n{PROJECT_ITEMS_FRAGMENT}"),
                ProjectVariables {
                    project_id: None,
                    owner: Some(owner.as_str()),
                    number: Some(*number),
                    first,
                    after,
                },
            ),
            ProjectLocator::User { owner, number } => (
                format!("{PROJECT_BY_USER_QUERY}\n{PROJECT_ITEMS_FRAGMENT}"),
                ProjectVariables {
                    project_id: None,
                    owner: Some(owner.as_str()),
                    number: Some(*number),
                    first,
                    after,
                },
            ),
        };
        Self { query, variables }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectVariables<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) project_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) owner: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) number: Option<i64>,
    pub(super) first: usize,
    pub(super) after: Option<&'a str>,
}

#[derive(Debug, Default)]
pub(super) struct ProjectItemsPage {
    pub(super) items: Vec<StreamItemUpsert>,
    pub(super) has_next_page: bool,
    pub(super) end_cursor: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct ProjectResponse {
    pub(super) data: Option<ProjectData>,
    pub(super) errors: Option<Vec<ProjectError>>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct ProjectError {
    pub(super) message: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectData {
    pub(super) node: Option<Project>,
    pub(super) organization: Option<ProjectOwner>,
    pub(super) user: Option<ProjectOwner>,
}

impl ProjectData {
    pub(super) fn project(self, locator: &ProjectLocator) -> Option<Project> {
        match locator {
            ProjectLocator::NodeId(_) => self.node,
            ProjectLocator::Organization { .. } => {
                self.organization.and_then(|owner| owner.project_v2)
            }
            ProjectLocator::User { .. } => self.user.and_then(|owner| owner.project_v2),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectOwner {
    pub(super) project_v2: Option<Project>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct Project {
    pub(super) items: Option<ProjectItems>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectItems {
    pub(super) page_info: PageInfo,
    pub(super) nodes: Vec<Option<ProjectItem>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PageInfo {
    pub(super) has_next_page: bool,
    pub(super) end_cursor: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectItem {
    pub(super) is_archived: bool,
    pub(super) updated_at: String,
    pub(super) content: Option<ProjectItemContent>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectItemContent {
    #[serde(rename = "__typename")]
    pub(super) typename: String,
    pub(super) id: String,
    pub(super) number: i64,
    pub(super) title: String,
    pub(super) url: String,
    pub(super) state: String,
    #[serde(default)]
    pub(super) is_draft: Option<bool>,
    #[serde(default)]
    pub(super) merged: Option<bool>,
    pub(super) merged_at: Option<String>,
    pub(super) created_at: String,
    pub(super) updated_at: String,
    pub(super) closed_at: Option<String>,
    pub(super) comments: CommentCount,
    pub(super) repository: ProjectRepository,
    pub(super) author: Option<ProjectUser>,
    #[serde(default)]
    pub(super) labels: ProjectLabels,
    #[serde(default)]
    pub(super) assignees: ProjectAssignees,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectRepository {
    pub(super) name_with_owner: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectUser {
    pub(super) login: String,
    pub(super) avatar_url: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(super) struct ProjectLabels {
    #[serde(default)]
    pub(super) nodes: Vec<Option<ProjectLabel>>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct ProjectLabel {
    pub(super) name: String,
}

#[derive(Debug, Default, serde::Deserialize)]
pub(super) struct ProjectAssignees {
    #[serde(default)]
    pub(super) nodes: Vec<Option<ProjectUser>>,
}

#[derive(Debug, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CommentCount {
    pub(super) total_count: i64,
}
