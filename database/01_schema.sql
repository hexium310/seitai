CREATE SCHEMA seitai AUTHORIZATION seitai;

CREATE TABLE "speakers" (
    "id" int NOT NULL,
    "speed" float4,
    PRIMARY KEY ("id")
);

CREATE TABLE "users" (
    "id" bigint NOT NULL,
    "speaker_id" int NOT NULL,
    UNIQUE ("id")
);

CREATE INDEX ON "users" ("id");
CREATE INDEX ON "speakers" ("id");
