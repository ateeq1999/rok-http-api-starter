use async_trait::async_trait;
use sqlx::postgres::PgRow;
use sqlx::PgPool;
use sqlx::FromRow;

#[derive(Debug, Clone)]
pub enum FieldValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    DateTime(chrono::DateTime<chrono::Utc>),
    OptionString(Option<String>),
    Null,
}

impl From<String> for FieldValue {
    fn from(s: String) -> Self {
        FieldValue::String(s)
    }
}

impl From<&str> for FieldValue {
    fn from(s: &str) -> Self {
        FieldValue::String(s.to_string())
    }
}

impl From<i64> for FieldValue {
    fn from(i: i64) -> Self {
        FieldValue::Int(i)
    }
}

impl From<f64> for FieldValue {
    fn from(f: f64) -> Self {
        FieldValue::Float(f)
    }
}

impl From<bool> for FieldValue {
    fn from(b: bool) -> Self {
        FieldValue::Bool(b)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for FieldValue {
    fn from(dt: chrono::DateTime<chrono::Utc>) -> Self {
        FieldValue::DateTime(dt)
    }
}

#[async_trait]
pub trait CrudService: Sized + for<'r> FromRow<'r, PgRow> + Send + Sync + Unpin {
    const TABLE: &'static str;
    const PK: &'static str = "id";

    async fn find_by_id(pool: &PgPool, id: &str) -> Result<Option<Self>, sqlx::Error> {
        let sql = format!("SELECT * FROM {} WHERE {} = $1", Self::TABLE, Self::PK);
        sqlx::query_as::<_, Self>(&sql)
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    async fn find_or_fail(pool: &PgPool, id: &str) -> Result<Self, sqlx::Error> {
        Self::find_by_id(pool, id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)
    }

    async fn all(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        let sql = format!("SELECT * FROM {}", Self::TABLE);
        sqlx::query_as::<_, Self>(&sql).fetch_all(pool).await
    }

    #[allow(dead_code)]
    async fn paginate(pool: &PgPool, page: i64, limit: i64) -> Result<(Vec<Self>, i64), sqlx::Error> {
        let offset = (page - 1).max(0) * limit;
        let sql = format!(
            "SELECT * FROM {} ORDER BY {} LIMIT $1 OFFSET $2",
            Self::TABLE,
            Self::PK
        );
        let count_sql = format!("SELECT COUNT(*) FROM {}", Self::TABLE);
        let total: (i64,) = sqlx::query_as(&count_sql).fetch_one(pool).await?;
        let items = sqlx::query_as::<_, Self>(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await?;
        Ok((items, total.0))
    }

    async fn delete(pool: &PgPool, id: &str) -> Result<bool, sqlx::Error> {
        let sql = format!("DELETE FROM {} WHERE {} = $1", Self::TABLE, Self::PK);
        let result = sqlx::query(&sql).bind(id).execute(pool).await?;
        Ok(result.rows_affected() > 0)
    }

    async fn create(pool: &PgPool, fields: &[(&str, FieldValue)]) -> Result<Self, sqlx::Error> {
        let cols: Vec<&str> = fields.iter().map(|(c, _)| *c).collect();
        let params: Vec<String> = (1..=fields.len()).map(|i| format!("${}", i)).collect();
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING *",
            Self::TABLE,
            cols.join(", "),
            params.join(", "),
        );
        let mut query = sqlx::query_as::<_, Self>(&sql);
        for (_, val) in fields {
            match val {
                FieldValue::String(s) => query = query.bind(s.as_str()),
                FieldValue::Int(i) => query = query.bind(*i),
                FieldValue::Float(f) => query = query.bind(*f),
                FieldValue::Bool(b) => query = query.bind(*b),
                FieldValue::DateTime(dt) => query = query.bind(dt),
                FieldValue::OptionString(None) => query = query.bind(None::<&str>),
                FieldValue::OptionString(Some(s)) => query = query.bind(s.as_str()),
                FieldValue::Null => query = query.bind(None::<&str>),
            }
        }
        query.fetch_one(pool).await
    }

    async fn update(pool: &PgPool, id: &str, fields: &[(&str, FieldValue)]) -> Result<Self, sqlx::Error> {
        let sets: Vec<String> = fields
            .iter()
            .enumerate()
            .map(|(i, (c, _))| format!("{} = ${}", c, i + 2))
            .collect();
        let sql = format!(
            "UPDATE {} SET {} WHERE {} = $1 RETURNING *",
            Self::TABLE,
            sets.join(", "),
            Self::PK,
        );
        let mut query = sqlx::query_as::<_, Self>(&sql).bind(id);
        for (_, val) in fields {
            match val {
                FieldValue::String(s) => query = query.bind(s.as_str()),
                FieldValue::Int(i) => query = query.bind(*i),
                FieldValue::Float(f) => query = query.bind(*f),
                FieldValue::Bool(b) => query = query.bind(*b),
                FieldValue::DateTime(dt) => query = query.bind(dt),
                FieldValue::OptionString(None) => query = query.bind(None::<&str>),
                FieldValue::OptionString(Some(s)) => query = query.bind(s.as_str()),
                FieldValue::Null => query = query.bind(None::<&str>),
            }
        }
        query.fetch_one(pool).await
    }
}
