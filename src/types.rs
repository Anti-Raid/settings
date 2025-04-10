use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;

pub type Error = Box<dyn std::error::Error + Send + Sync>; // This is constant and should be copy pasted

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub enum ColumnType {
    /// A single valued column (scalar)
    Scalar {
        /// The value type
        inner: InnerColumnType,
    },
    /// An array column
    Array {
        /// The inner type of the array
        inner: InnerColumnType,
    },
}

impl ColumnType {
    /// Returns whether the column type is an array
    #[allow(dead_code)]
    pub fn is_array(&self) -> bool {
        matches!(self, ColumnType::Array { .. })
    }

    /// Returns whether the column type is a scalar
    #[allow(dead_code)]
    pub fn is_scalar(&self) -> bool {
        matches!(self, ColumnType::Scalar { .. })
    }

    pub fn new_scalar(inner: InnerColumnType) -> Self {
        ColumnType::Scalar { inner }
    }

    pub fn new_array(inner: InnerColumnType) -> Self {
        ColumnType::Array { inner }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub enum InnerColumnType {
    String {
        min_length: Option<usize>,
        max_length: Option<usize>,
        allowed_values: Vec<String>, // If empty, all values are allowed
        kind: String, // e.g. uuid, textarea, channel, user, role, interval, timestamp etc.
    },
    Integer {},
    Float {},
    BitFlag {
        /// The bit flag values
        values: indexmap::IndexMap<String, i64>,
    },
    Boolean {},
    Json {
        kind: String, // e.g. templateref etc.
        max_bytes: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ColumnSuggestion {
    Static { suggestions: Vec<String> },
    None {},
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Column {
    /// The ID of the column on the database
    pub id: String,

    /// The friendly name of the column
    pub name: String,

    /// The description of the column
    pub description: String,

    /// The type of the column
    pub column_type: ColumnType,

    /// Whether or not the column is a primary key
    pub primary_key: bool,

    /// Whether or not the column is nullable
    ///
    /// Note that the point where nullability is checked may vary but will occur after pre_checks are executed
    pub nullable: bool,

    /// Suggestions to display
    pub suggestions: ColumnSuggestion,

    /// A secret field that is not shown to the user
    pub secret: bool,

    /// For which operations should the field be ignored for (essentially, read only)
    ///
    /// Semantics are defined by the Executor
    pub ignored_for: Vec<OperationType>,
}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub enum OperationType {
    View,
    Create,
    Update,
    Delete,
}

impl std::fmt::Display for OperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OperationType::View => write!(f, "View"),
            OperationType::Create => write!(f, "Create"),
            OperationType::Update => write!(f, "Update"),
            OperationType::Delete => write!(f, "Delete"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Setting<SettingsData: Clone> {
    /// The ID of the option
    pub id: String,

    /// The name of the option
    pub name: String,

    /// The description of the option
    pub description: String,

    /// Title template, used for the title of the embed
    pub title_template: String,

    /// The columns for this option
    pub columns: Arc<Vec<Column>>,

    /// The supported operations for this option
    #[serde(skip_deserializing)]
    pub operations: SettingOperations<SettingsData>,
}

#[derive(Clone, Default)]
pub struct SettingOperations<SettingsData: Clone> {
    /// How to view this setting
    pub view: Option<Arc<dyn SettingView<SettingsData>>>,

    /// How to create this setting
    pub create: Option<Arc<dyn SettingCreator<SettingsData>>>,

    /// How to update this setting
    pub update: Option<Arc<dyn SettingUpdater<SettingsData>>>,

    /// How to delete this setting
    pub delete: Option<Arc<dyn SettingDeleter<SettingsData>>>,
}

impl<SettingsData: Clone> std::fmt::Debug for SettingOperations<SettingsData> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SettingOperations")
    }
}

impl<SettingsData: Clone> serde::Serialize for SettingOperations<SettingsData> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut supported_operations = Vec::new();

        if let Some(_v) = &self.view {
            supported_operations.push(OperationType::View);
        }

        if let Some(_v) = &self.create {
            supported_operations.push(OperationType::Create);
        }

        if let Some(_v) = &self.update {
            supported_operations.push(OperationType::Update);
        }

        if let Some(_v) = &self.delete {
            supported_operations.push(OperationType::Delete);
        }

        supported_operations.serialize(serializer)
    }
}

impl<SettingsData: Clone> PartialEq for Setting<SettingsData> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// Wraps `v` in the currently used wrapper
///
/// Currently, this is an Arc for now
pub fn settings_wrap<T>(v: T) -> Arc<T> {
    Arc::new(v)
}

#[async_trait]
pub trait SettingView<SettingsData: Clone>: Send + Sync {
    /// View the settings data
    ///
    /// All Executors should return an __count value containing the total count of the total number of entries
    async fn view<'a>(
        &self,
        context: &SettingsData,
        filters: indexmap::IndexMap<String, Value>,
    ) -> Result<Vec<indexmap::IndexMap<String, Value>>, Error>;
}

#[async_trait]
pub trait SettingCreator<SettingsData: Clone>: Send + Sync {
    /// Creates the setting
    async fn create<'a>(
        &self,
        context: &SettingsData,
        state: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error>;
}

#[async_trait]
pub trait SettingUpdater<SettingsData: Clone>: Send + Sync {
    /// Updates the setting
    async fn update<'a>(
        &self,
        context: &SettingsData,
        state: indexmap::IndexMap<String, Value>,
    ) -> Result<indexmap::IndexMap<String, Value>, Error>;
}

#[async_trait]
pub trait SettingDeleter<SettingsData: Clone>: Send + Sync {
    /// Deletes the setting
    async fn delete<'a>(&self, context: &SettingsData, state: indexmap::IndexMap<String, Value>) -> Result<(), Error>;
}

impl<SettingsData: Clone> SettingOperations<SettingsData> {
    pub fn from<U>(v: U) -> Self
    where
        U: SettingView<SettingsData>
            + SettingCreator<SettingsData>
            + SettingUpdater<SettingsData>
            + SettingDeleter<SettingsData>
            + Clone
            + 'static,
    {
        SettingOperations {
            view: Some(settings_wrap(v.clone())),
            create: Some(settings_wrap(v.clone())),
            update: Some(settings_wrap(v.clone())),
            delete: Some(settings_wrap(v)),
        }
    }
}

#[allow(dead_code)]
impl<SettingsData: Clone> SettingOperations<SettingsData> {
    pub fn to_view_op<T: SettingView<SettingsData> + Clone + 'static>(v: T) -> Self {
        SettingOperations {
            view: Some(settings_wrap(v)),
            create: None,
            update: None,
            delete: None,
        }
    }

    pub fn to_create_op<T: SettingCreator<SettingsData> + Clone + 'static>(v: T) -> Self {
        SettingOperations {
            view: None,
            create: Some(settings_wrap(v)),
            update: None,
            delete: None,
        }
    }

    pub fn to_update_op<T: SettingUpdater<SettingsData> + Clone + 'static>(v: T) -> Self {
        SettingOperations {
            view: None,
            create: None,
            update: Some(settings_wrap(v)),
            delete: None,
        }
    }

    pub fn to_delete_op<T: SettingDeleter<SettingsData> + Clone + 'static>(v: T) -> Self {
        SettingOperations {
            view: None,
            create: None,
            update: None,
            delete: Some(settings_wrap(v)),
        }
    }

    pub fn to_view_create_op<
        T: SettingView<SettingsData> + SettingCreator<SettingsData> + Clone + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: Some(settings_wrap(v.clone())),
            create: Some(settings_wrap(v)),
            update: None,
            delete: None,
        }
    }

    pub fn to_view_update_op<
        T: SettingView<SettingsData> + SettingUpdater<SettingsData> + Clone + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: Some(settings_wrap(v.clone())),
            create: None,
            update: Some(settings_wrap(v)),
            delete: None,
        }
    }

    pub fn to_view_delete_op<
        T: SettingView<SettingsData> + SettingDeleter<SettingsData> + Clone + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: Some(settings_wrap(v.clone())),
            create: None,
            update: None,
            delete: Some(settings_wrap(v)),
        }
    }

    pub fn to_create_update_op<
        T: SettingCreator<SettingsData> + SettingUpdater<SettingsData> + Clone + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: None,
            create: Some(settings_wrap(v.clone())),
            update: Some(settings_wrap(v)),
            delete: None,
        }
    }

    pub fn to_create_delete_op<
        T: SettingCreator<SettingsData> + SettingDeleter<SettingsData> + Clone + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: None,
            create: Some(settings_wrap(v.clone())),
            update: None,
            delete: Some(settings_wrap(v)),
        }
    }

    pub fn to_update_delete_op<
        T: SettingUpdater<SettingsData> + SettingDeleter<SettingsData> + Clone + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: None,
            create: None,
            update: Some(settings_wrap(v.clone())),
            delete: Some(settings_wrap(v)),
        }
    }

    pub fn to_view_create_update_op<
        T: SettingView<SettingsData>
            + SettingCreator<SettingsData>
            + SettingUpdater<SettingsData>
            + Clone
            + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: Some(settings_wrap(v.clone())),
            create: Some(settings_wrap(v.clone())),
            update: Some(settings_wrap(v)),
            delete: None,
        }
    }

    pub fn to_view_create_delete_op<
        T: SettingView<SettingsData>
            + SettingCreator<SettingsData>
            + SettingDeleter<SettingsData>
            + Clone
            + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: Some(settings_wrap(v.clone())),
            create: Some(settings_wrap(v.clone())),
            update: None,
            delete: Some(settings_wrap(v)),
        }
    }

    pub fn to_view_update_delete_op<
        T: SettingView<SettingsData>
            + SettingUpdater<SettingsData>
            + SettingDeleter<SettingsData>
            + Clone
            + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: Some(settings_wrap(v.clone())),
            create: None,
            update: Some(settings_wrap(v.clone())),
            delete: Some(settings_wrap(v)),
        }
    }

    pub fn to_create_update_delete_op<
        T: SettingCreator<SettingsData>
            + SettingUpdater<SettingsData>
            + SettingDeleter<SettingsData>
            + Clone
            + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: None,
            create: Some(settings_wrap(v.clone())),
            update: Some(settings_wrap(v.clone())),
            delete: Some(settings_wrap(v)),
        }
    }

    pub fn to_view_create_update_delete_op<
        T: SettingView<SettingsData>
            + SettingCreator<SettingsData>
            + SettingUpdater<SettingsData>
            + SettingDeleter<SettingsData>
            + Clone
            + 'static,
    >(
        v: T,
    ) -> Self {
        SettingOperations {
            view: Some(settings_wrap(v.clone())),
            create: Some(settings_wrap(v.clone())),
            update: Some(settings_wrap(v.clone())),
            delete: Some(settings_wrap(v)),
        }
    }
}
