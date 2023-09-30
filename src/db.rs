use diesel::r2d2::ConnectionManager;
use diesel::PgConnection;
use r2d2;
use rocket::http::Status;
use rocket::request::{self, FromRequest};
use rocket::{Outcome, Request, State};
use std::ops::{Deref, DerefMut};

pub type Pool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub fn init_pool(db_url: String) -> Pool {
	let manager = ConnectionManager::<PgConnection>::new(db_url);
	r2d2::Pool::builder()
		.build(manager)
		.expect("failed to build pool")
}

pub struct Conn(pub r2d2::PooledConnection<ConnectionManager<PgConnection>>);

impl<'a, 'r> FromRequest<'a, 'r> for Conn {
	type Error = ();

	fn from_request(request: &'a Request<'r>) -> request::Outcome<Conn, ()> {
		let pool = request.guard::<State<Pool>>()?;
		match pool.get() {
			Ok(conn) => Outcome::Success(Conn(conn)),
			Err(_) => Outcome::Failure((Status::ServiceUnavailable, ())),
		}
	}
}

impl Deref for Conn {
	type Target = PgConnection;

	#[inline(always)]
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl DerefMut for Conn {
	#[inline(always)]
	fn deref_mut(self: &mut Conn) -> &mut diesel::PgConnection {
		&mut self.0
	}
}
