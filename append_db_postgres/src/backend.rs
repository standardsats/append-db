use crate::update::{HasUpdateTag, UpdateBodyError, VersionedState};
pub use append_db::backend::class::{SnapshotedUpdate, State, StateBackend};
use async_trait::async_trait;
use chrono::prelude::*;
use futures::StreamExt;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::Mutex;
use sqlx::Row;

/// Connection pool to Postgres
pub type Pool = sqlx::Pool<sqlx::Postgres>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Failed to decode body by tag: {0}")]
    UpdateBody(#[from] UpdateBodyError),
    #[error("Failed to decode/encode JSON: {0}")]
    Encoding(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct Postgres<St: State> {
    pub pool: Arc<Mutex<Pool>>,
    pub state_proxy: PhantomData<St>,
}

impl<St: State> Postgres<St> {
    pub fn new(pool: Pool) -> Self {
        Postgres {
            pool: Arc::new(Mutex::new(pool)),
            state_proxy: PhantomData,
        }
    }
}

#[async_trait]
impl<
        Upd: HasUpdateTag + Send,
        St: State<Update = Upd> + VersionedState + Clone + Send + Sync + 'static,
    > StateBackend for Postgres<St>
{
    type State = St;
    type Err = Error;

    async fn write(&mut self, update: SnapshotedUpdate<St>) -> Result<(), Self::Err> {
        let now = Utc::now().naive_utc();
        let tag = format!("{}", update.get_tag());
        let body = update.serialize_untagged()?;
        let pool = self.pool.lock().await;
        let query = format!("insert into {} (created, version, tag, body) values ({}, {}, {}, {})",
            St::TABLE,
            now,
            update.get_version() as i16,
            tag,
            body,
        );
        println!("{}", query);
        sqlx::query(&query)
        // sqlx::query!(
        //     "insert into updates (created, version, tag, body) values ($1, $2, $3, $4)",
        //     now,
        //     update.get_version() as i16,
        //     tag,
        //     body,
        // )
        .execute(pool.deref())
        .await?;
        Ok(())
    }

    async fn updates(&self) -> Result<Vec<SnapshotedUpdate<St>>, Self::Err> {
        let pool = self.pool.lock().await;
        let mut conn = pool.acquire().await?;
        let query = format!("select from {} order by created desc", St::TABLE);
        let res = sqlx::query(&query)
        // let res = sqlx::query!("select * from $1 order by created desc", St::TABLE)
            .fetch(&mut conn)
            .fuse();
        futures::pin_mut!(res);
        let mut parsed: Vec<SnapshotedUpdate<St>> = vec![];
        loop {
            let item = futures::select! {
                mmrow = res.next() => {
                    if let Some(mrow) = mmrow {
                        let r = mrow?;
                        let body = <SnapshotedUpdate<St>>::deserialize_by_tag(
                            &Cow::Owned(r.get("tag")), 
                            r.get::<i16, &str>("version") as u16, 
                            r.get::<serde_json::Value, &str>(("body").clone())
                        )?;
                        body
                    } else {
                        break;
                    }
                },
                complete => break,
            };
            let is_end = item.is_snapshot();
            parsed.push(item);
            if is_end {
                break;
            }
        }
        parsed.reverse();
        Ok(parsed)
    }
}
