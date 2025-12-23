use macros::query;

fn main() {
    // jchen: print TokenStream; define hello function
    // query!(SELECT * FROM users WHERE age > 10);
    query!(SELECT * FROM users u JOIN (SELECT * from profiles p) WHERE u.id = p.id and u.age > 10);
    // then call function
    hello()
}
