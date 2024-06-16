use crate::schema::users;
use crate::schema::users::dsl::users as all_users;
use diesel::prelude::*;
use diesel::PgConnection;

#[derive(Serialize, Queryable)]
pub struct User {
	pub id: i32,
	pub email: String,
	pub password: String,
	pub first_name: String,
	pub last_name: String,
}
#[derive(Serialize, Deserialize)]
pub struct Me {
	pub id: i32,
	pub email: String,
	pub first_name: String,
	pub last_name: String,
}
// decode request data
#[derive(Deserialize)]
pub struct UserData {
	pub email: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LoginUser {
	pub email: String,
	pub password: String,
}
#[derive(Serialize, Deserialize, Clone)]
pub struct RegisterUser {
	pub email: String,
	pub password: String,
	pub rep_password: String,
	pub first_name: String,
	pub last_name: String,
}

// this is to insert users to database
#[derive(Serialize, Deserialize, Insertable, Clone, Selectable)]
#[table_name = "users"]
pub struct NewUser {
	pub email: String,
	pub password: String,
	pub first_name: String,
	pub last_name: String,
}
impl std::convert::From<RegisterUser> for NewUser {
	fn from(value: RegisterUser) -> Self {
		Self {
			email: value.email,
			password: value.password,
			first_name: value.first_name,
			last_name: value.last_name,
		}
	}
}
impl User {
	pub fn get_all_users(conn: &mut PgConnection) -> Vec<User> {
		all_users
			.order(users::id.desc())
			.load::<User>(conn)
			.expect("error!")
	}

	pub fn insert_user(
		user: NewUser,
		conn: &mut PgConnection,
	) -> Result<usize, diesel::result::Error> {
		diesel::insert_into(users::table)
			.values(&user)
			.execute(conn)
	}

	pub fn get_user_by_email(email: String, conn: &mut PgConnection) -> Vec<User> {
		diesel::query_dsl::methods::FilterDsl::filter(all_users, users::email.eq(email))
			.load::<User>(conn)
			.expect("error!")
	}
	pub fn get_user_by_id(id: i32, conn: &mut PgConnection) -> Vec<User> {
		diesel::query_dsl::methods::FilterDsl::filter(all_users, users::id.eq(id))
			.load::<User>(conn)
			.expect("error!")
	}
}
