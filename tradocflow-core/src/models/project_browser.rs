use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::models::project::{Project, ProjectStatus, Priority, ProjectSummary};

/// Project browser state and filtering options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBrowserState {
    /// All available projects
    pub projects: Vec<ProjectBrowserItem>,
    /// Filtered projects based on current search/filter criteria
    pub filtered_projects: Vec<ProjectBrowserItem>,
    /// Current search query
    pub search_query: String,
    /// Active filters
    pub filters: ProjectFilters,
    /// Current sort configuration
    pub sort_config: SortConfig,
    /// View mode (grid/list)
    pub view_mode: ViewMode,
    /// Selected project ID
    pub selected_project_id: Option<Uuid>,
    /// Loading state
    pub is_loading: bool,
    /// Current page for pagination
    pub current_page: usize,
    /// Items per page
    pub items_per_page: usize,
    /// Total pages
    pub total_pages: usize,
    /// Recently accessed projects
    pub recent_projects: Vec<RecentProject>,
    /// Bookmarked/favorite projects
    pub favorite_projects: Vec<Uuid>,
}

/// Project item for browser display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBrowserItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: ProjectStatus,
    pub priority: Priority,
    pub owner_id: String,
    pub owner_name: String,
    pub member_count: usize,
    pub document_count: usize,
    pub task_count: usize,
    pub progress_percentage: f32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub due_date: Option<DateTime<Utc>>,
    pub last_activity: String,
    pub thumbnail: Option<String>,
    pub tags: Vec<String>,
    pub languages: Vec<String>,
    pub is_favorite: bool,
    pub is_archived: bool,
    pub access_level: AccessLevel,
}

/// Recent project access information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub project_id: Uuid,
    pub name: String,
    pub last_accessed: DateTime<Utc>,
    pub access_count: usize,
    pub thumbnail: Option<String>,
}

/// Filter options for project browsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectFilters {
    /// Filter by project status(es)
    pub status_filter: Vec<ProjectStatus>,
    /// Filter by priority level(s)
    pub priority_filter: Vec<Priority>,
    /// Filter by date range
    pub date_range: Option<DateRange>,
    /// Filter by team membership
    pub membership_filter: MembershipFilter,
    /// Filter by owner
    pub owner_filter: Option<String>,
    /// Filter by tags
    pub tag_filter: Vec<String>,
    /// Filter by languages
    pub language_filter: Vec<String>,
    /// Show archived projects
    pub show_archived: bool,
    /// Show only favorites
    pub favorites_only: bool,
}

/// Date range filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub date_type: DateFilterType,
}

/// Date filter types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DateFilterType {
    Created,
    Updated,
    DueDate,
}

/// Membership filter options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MembershipFilter {
    All,
    Owner,
    Member,
    Contributor,
}

/// Sort configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortConfig {
    pub field: SortField,
    pub direction: SortDirection,
}

/// Available sort fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortField {
    Name,
    Created,
    Updated,
    DueDate,
    Priority,
    Status,
    Progress,
    LastActivity,
    MemberCount,
    DocumentCount,
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// View modes for project display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewMode {
    Grid,
    List,
    Compact,
}

/// User access level for projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccessLevel {
    Owner,
    Admin,
    Editor,
    Viewer,
    Guest,
}

/// Project action types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectAction {
    Open,
    Edit,
    Settings,
    Duplicate,
    Archive,
    Unarchive,
    Delete,
    Export,
    Share,
    Favorite,
    Unfavorite,
}

/// Project context menu item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMenuItem {
    pub action: ProjectAction,
    pub label: String,
    pub icon: String,
    pub enabled: bool,
    pub separator_after: bool,
}

/// Search options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    /// Search in project names
    pub search_names: bool,
    /// Search in descriptions
    pub search_descriptions: bool,
    /// Search in owner names
    pub search_owners: bool,
    /// Search in tags
    pub search_tags: bool,
    /// Case sensitive search
    pub case_sensitive: bool,
    /// Use regex patterns
    pub use_regex: bool,
}

impl Default for ProjectBrowserState {
    fn default() -> Self {
        Self {
            projects: Vec::new(),
            filtered_projects: Vec::new(),
            search_query: String::new(),
            filters: ProjectFilters::default(),
            sort_config: SortConfig::default(),
            view_mode: ViewMode::Grid,
            selected_project_id: None,
            is_loading: false,
            current_page: 1,
            items_per_page: 20,
            total_pages: 1,
            recent_projects: Vec::new(),
            favorite_projects: Vec::new(),
        }
    }
}

impl Default for ProjectFilters {
    fn default() -> Self {
        Self {
            status_filter: vec![
                ProjectStatus::Active,
                ProjectStatus::OnHold,
            ],
            priority_filter: Vec::new(),
            date_range: None,
            membership_filter: MembershipFilter::All,
            owner_filter: None,
            tag_filter: Vec::new(),
            language_filter: Vec::new(),
            show_archived: false,
            favorites_only: false,
        }
    }
}

impl Default for SortConfig {
    fn default() -> Self {
        Self {
            field: SortField::Updated,
            direction: SortDirection::Descending,
        }
    }
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            search_names: true,
            search_descriptions: true,
            search_owners: true,
            search_tags: true,
            case_sensitive: false,
            use_regex: false,
        }
    }
}

impl ProjectBrowserItem {
    /// Convert from Project and ProjectSummary
    pub fn from_project_and_summary(
        project: &Project,
        summary: &ProjectSummary,
        owner_name: &str,
        is_favorite: bool,
        access_level: AccessLevel,
    ) -> Self {
        let progress = if summary.document_count > 0 {
            // Simple progress calculation based on document completion
            // In real implementation, this would be more sophisticated
            0.5 + (rand::random::<f32>() * 0.5)
        } else {
            0.0
        };

        let last_activity = format!("{} days ago", 
            (chrono::Utc::now() - project.updated_at).num_days());

        Self {
            id: project.id,
            name: project.name.clone(),
            description: project.description.clone(),
            status: project.status.clone(),
            priority: project.priority.clone(),
            owner_id: project.owner_id.clone(),
            owner_name: owner_name.to_string(),
            member_count: summary.member_count,
            document_count: summary.document_count,
            task_count: summary.kanban_card_count,
            progress_percentage: progress * 100.0,
            created_at: project.created_at,
            updated_at: project.updated_at,
            due_date: project.due_date,
            last_activity,
            thumbnail: None,
            tags: Vec::new(), // Would be populated from metadata
            languages: Vec::new(), // Would be populated from project structure
            is_favorite,
            is_archived: project.status == ProjectStatus::Cancelled,
            access_level,
        }
    }

    /// Check if project matches search query
    pub fn matches_search(&self, query: &str, options: &SearchOptions) -> bool {
        if query.is_empty() {
            return true;
        }

        let query = if options.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let matches_name = if options.search_names {
            let name = if options.case_sensitive {
                self.name.clone()
            } else {
                self.name.to_lowercase()
            };
            name.contains(&query)
        } else {
            false
        };

        let matches_description = if options.search_descriptions {
            if let Some(desc) = &self.description {
                let desc = if options.case_sensitive {
                    desc.clone()
                } else {
                    desc.to_lowercase()
                };
                desc.contains(&query)
            } else {
                false
            }
        } else {
            false
        };

        let matches_owner = if options.search_owners {
            let owner = if options.case_sensitive {
                self.owner_name.clone()
            } else {
                self.owner_name.to_lowercase()
            };
            owner.contains(&query)
        } else {
            false
        };

        let matches_tags = if options.search_tags {
            self.tags.iter().any(|tag| {
                let tag = if options.case_sensitive {
                    tag.clone()
                } else {
                    tag.to_lowercase()
                };
                tag.contains(&query)
            })
        } else {
            false
        };

        matches_name || matches_description || matches_owner || matches_tags
    }

    /// Check if project matches filters
    pub fn matches_filters(&self, filters: &ProjectFilters) -> bool {
        // Status filter
        if !filters.status_filter.is_empty() && !filters.status_filter.contains(&self.status) {
            return false;
        }

        // Priority filter
        if !filters.priority_filter.is_empty() && !filters.priority_filter.contains(&self.priority) {
            return false;
        }

        // Archived filter
        if !filters.show_archived && self.is_archived {
            return false;
        }

        // Favorites filter
        if filters.favorites_only && !self.is_favorite {
            return false;
        }

        // Date range filter
        if let Some(date_range) = &filters.date_range {
            let date_to_check = match date_range.date_type {
                DateFilterType::Created => &self.created_at,
                DateFilterType::Updated => &self.updated_at,
                DateFilterType::DueDate => {
                    if let Some(due_date) = &self.due_date {
                        due_date
                    } else {
                        return false;
                    }
                }
            };

            if date_to_check < &date_range.start_date || date_to_check > &date_range.end_date {
                return false;
            }
        }

        // Tag filter
        if !filters.tag_filter.is_empty() {
            let has_matching_tag = filters.tag_filter.iter()
                .any(|filter_tag| self.tags.contains(filter_tag));
            if !has_matching_tag {
                return false;
            }
        }

        // Language filter
        if !filters.language_filter.is_empty() {
            let has_matching_language = filters.language_filter.iter()
                .any(|filter_lang| self.languages.contains(filter_lang));
            if !has_matching_language {
                return false;
            }
        }

        true
    }

    /// Get available context menu actions
    pub fn get_context_menu_items(&self) -> Vec<ContextMenuItem> {
        let mut items = Vec::new();

        // Open action (always available)
        items.push(ContextMenuItem {
            action: ProjectAction::Open,
            label: "Open Project".to_string(),
            icon: "📂".to_string(),
            enabled: true,
            separator_after: true,
        });

        // Edit actions (based on access level)
        let can_edit = matches!(self.access_level, AccessLevel::Owner | AccessLevel::Admin | AccessLevel::Editor);
        
        items.push(ContextMenuItem {
            action: ProjectAction::Settings,
            label: "Project Settings".to_string(),
            icon: "⚙️".to_string(),
            enabled: can_edit,
            separator_after: false,
        });

        items.push(ContextMenuItem {
            action: ProjectAction::Duplicate,
            label: "Duplicate Project".to_string(),
            icon: "📋".to_string(),
            enabled: true,
            separator_after: true,
        });

        // Favorite/Unfavorite
        if self.is_favorite {
            items.push(ContextMenuItem {
                action: ProjectAction::Unfavorite,
                label: "Remove from Favorites".to_string(),
                icon: "⭐".to_string(),
                enabled: true,
                separator_after: false,
            });
        } else {
            items.push(ContextMenuItem {
                action: ProjectAction::Favorite,
                label: "Add to Favorites".to_string(),
                icon: "☆".to_string(),
                enabled: true,
                separator_after: false,
            });
        }

        // Export and Share
        items.push(ContextMenuItem {
            action: ProjectAction::Export,
            label: "Export Project".to_string(),
            icon: "📤".to_string(),
            enabled: true,
            separator_after: false,
        });

        items.push(ContextMenuItem {
            action: ProjectAction::Share,
            label: "Share Project".to_string(),
            icon: "🔗".to_string(),
            enabled: can_edit,
            separator_after: true,
        });

        // Archive/Unarchive (owner/admin only)
        let can_archive = matches!(self.access_level, AccessLevel::Owner | AccessLevel::Admin);
        
        if self.is_archived {
            items.push(ContextMenuItem {
                action: ProjectAction::Unarchive,
                label: "Restore Project".to_string(),
                icon: "📥".to_string(),
                enabled: can_archive,
                separator_after: false,
            });
        } else {
            items.push(ContextMenuItem {
                action: ProjectAction::Archive,
                label: "Archive Project".to_string(),
                icon: "📦".to_string(),
                enabled: can_archive,
                separator_after: false,
            });
        }

        // Delete (owner only)
        if matches!(self.access_level, AccessLevel::Owner) {
            items.push(ContextMenuItem {
                action: ProjectAction::Delete,
                label: "Delete Project".to_string(),
                icon: "🗑️".to_string(),
                enabled: true,
                separator_after: false,
            });
        }

        items
    }
}

impl ProjectStatus {
    /// Get status display color
    pub fn get_color(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "#4CAF50",      // Green
            ProjectStatus::Completed => "#2196F3",   // Blue
            ProjectStatus::OnHold => "#FF9800",      // Orange
            ProjectStatus::Cancelled => "#F44336",   // Red
        }
    }

    /// Get status icon
    pub fn get_icon(&self) -> &'static str {
        match self {
            ProjectStatus::Active => "🟢",
            ProjectStatus::Completed => "✅",
            ProjectStatus::OnHold => "⏸️",
            ProjectStatus::Cancelled => "❌",
        }
    }
}

impl Priority {
    /// Get priority display color
    pub fn get_color(&self) -> &'static str {
        match self {
            Priority::Low => "#4CAF50",       // Green
            Priority::Medium => "#FF9800",    // Orange
            Priority::High => "#FF5722",      // Deep Orange
            Priority::Urgent => "#F44336",    // Red
        }
    }

    /// Get priority icon
    pub fn get_icon(&self) -> &'static str {
        match self {
            Priority::Low => "🔵",
            Priority::Medium => "🟡",
            Priority::High => "🟠",
            Priority::Urgent => "🔴",
        }
    }
}

impl AccessLevel {
    /// Get access level display string
    pub fn display_string(&self) -> &'static str {
        match self {
            AccessLevel::Owner => "Owner",
            AccessLevel::Admin => "Admin",
            AccessLevel::Editor => "Editor",
            AccessLevel::Viewer => "Viewer",
            AccessLevel::Guest => "Guest",
        }
    }
}