use typedflake::typedflake;

#[typedflake(epoch = "2025-02-30")]
struct NotACalendarDate(i64);

#[typedflake(epoch = "2025-13-01")]
struct NotAMonth(i64);

#[typedflake(epoch = "1969-12-31")]
struct BeforeUnixEpoch(i64);

#[typedflake(epoch = "2025/01/01")]
struct WrongSeparator(i64);

#[typedflake(epoch = "2025-1-1")]
struct NotZeroPadded(i64);

#[typedflake(epoch = "2025-01-01T00:00:00Z")]
struct WithTime(i64);

#[typedflake(epoch = 1735689600000)]
struct UnixMillis(i64);

fn main() {}
