use crate::{
    db::{model, MangaDatabase},
    user::Claims,
};
use async_graphql::{Context, Object, Result};

#[derive(Debug, Clone)]
pub struct Category {
    id: i64,
    name: String,
}

impl From<model::Category> for Category {
    fn from(val: model::Category) -> Self {
        Self {
            id: val.id,
            name: val.name,
        }
    }
}

#[Object]
impl Category {
    async fn id(&self) -> i64 {
        self.id
    }

    async fn name(&self) -> String {
        self.name.clone()
    }

    async fn count(&self, ctx: &Context<'_>) -> Result<i64> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        let id = if self.id > 0 { Some(self.id) } else { None };
        Ok(ctx
            .data::<MangaDatabase>()?
            .count_library_by_category_id(user.sub, id)
            .await?)
    }
}

#[derive(Default)]
pub struct CategoryRoot;

#[Object]
impl CategoryRoot {
    async fn get_categories(&self, ctx: &Context<'_>) -> Result<Vec<Category>> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        let res = ctx
            .data::<MangaDatabase>()?
            .get_user_categories(user.sub)
            .await?;

        let mut categories = vec![Category {
            id: 0,
            name: "Default".to_string(),
        }];
        for item in res {
            categories.push(item.into());
        }

        Ok(categories)
    }

    async fn get_category(&self, ctx: &Context<'_>, id: i64) -> Result<Category> {
        let _ = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;

        if id == 0 {
            Ok(Category {
                id: 0,
                name: "Default".to_string(),
            })
        } else {
            let res = ctx.data::<MangaDatabase>()?.get_user_category(id).await?;

            Ok(res.into())
        }
    }
}

#[derive(Default)]
pub struct CategoryMutationRoot;

#[Object]
impl CategoryMutationRoot {
    async fn create_category(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "category name")] name: String,
    ) -> Result<u64> {
        let user = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;
        match ctx
            .data::<MangaDatabase>()?
            .insert_user_category(user.sub, &name)
            .await
        {
            Ok(rows) => Ok(rows),
            Err(err) => Err(format!("error create category: {}", err).into()),
        }
    }

    async fn update_category(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "category id")] id: i64,
        #[graphql(desc = "category name")] name: String,
    ) -> Result<u64> {
        let _ = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;
        match ctx
            .data::<MangaDatabase>()?
            .update_user_category(id, &name)
            .await
        {
            Ok(rows) => Ok(rows),
            Err(err) => Err(format!("error create category: {}", err).into()),
        }
    }

    async fn delete_category(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "category id")] id: i64,
    ) -> Result<u64> {
        let _ = ctx
            .data::<Claims>()
            .map_err(|_| "token not exists, please login")?;
        match ctx.data::<MangaDatabase>()?.delete_user_category(id).await {
            Ok(rows) => Ok(rows),
            Err(err) => Err(format!("error create category: {}", err).into()),
        }
    }
}
