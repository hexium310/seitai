CREATE TABLE "users" (
    "id" bigint NOT NULL,
    "speaker_id" int NOT NULL,
    UNIQUE ("id")
);

CREATE INDEX ON "users" ("id");
