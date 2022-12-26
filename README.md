# essence
Essential models and database operations used by other Adapt microservices.

## What does this service contain?
This is not a standalone service, but rather a library of common models and operations used by other Adapt 
microservices.

This includes:
* Object schemas (e.g. `User` or `Guild`)
  * This includes payload schemas, both inbound and outbound (e.g. `UserCreatePayload`).
* Database operations (e.g. `User::create` or `Guild::delete`)
* Snowflake generation
* Utilities that relate to the above
