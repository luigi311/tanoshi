pub mod settings {
    use crate::auth::Claims;
    use crate::settings::settings::Settings;
    use crate::settings::SettingParams;
    use sled::Db;
    use std::convert::Infallible;

    pub async fn set_settings(
        claim: Claims,
        param: SettingParams,
        settings: Settings,
        db: Db,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = settings.set(claim.sub, param, db);
        Ok(warp::reply::json(&res))
    }

    pub async fn get_settings(
        claim: Claims,
        settings: Settings,
        db: Db,
    ) -> Result<impl warp::Reply, Infallible> {
        let res = settings.get(claim.sub, db);
        Ok(warp::reply::json(&res))
    }
}
