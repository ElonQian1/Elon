CREATE TABLE IF NOT EXISTS game_reward_policy (
  singleton INTEGER PRIMARY KEY CHECK(singleton=1), digest TEXT NOT NULL UNIQUE,
  source_json TEXT NOT NULL CHECK(json_valid(source_json))
);
CREATE TABLE IF NOT EXISTS game_reward_reports (
  digest TEXT PRIMARY KEY NOT NULL, user_id TEXT NOT NULL REFERENCES users(id),
  policy_digest TEXT NOT NULL REFERENCES game_reward_policy(digest),
  sequence INTEGER NOT NULL CHECK(typeof(sequence)='integer' AND sequence>0),
  previous_digest TEXT NOT NULL, beneficiary TEXT NOT NULL,
  period_end_ms INTEGER NOT NULL CHECK(typeof(period_end_ms)='integer' AND period_end_ms>0),
  net_units INTEGER NOT NULL CHECK(typeof(net_units)='integer'),
  proof_json TEXT NOT NULL CHECK(json_valid(proof_json)),
  actor_id TEXT NOT NULL REFERENCES users(id), recorded_ms INTEGER NOT NULL,
  UNIQUE(user_id, sequence)
);
CREATE TABLE IF NOT EXISTS game_reward_intents (
  allocation_hash TEXT PRIMARY KEY NOT NULL, digest TEXT NOT NULL UNIQUE,
  user_id TEXT NOT NULL REFERENCES users(id), report_digest TEXT NOT NULL REFERENCES game_reward_reports(digest),
  amount_units INTEGER NOT NULL CHECK(typeof(amount_units)='integer' AND amount_units>0),
  intent_json TEXT NOT NULL CHECK(json_valid(intent_json)),
  actor_id TEXT NOT NULL REFERENCES users(id), recorded_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS game_reward_intents_user ON game_reward_intents(user_id,recorded_ms);
CREATE TABLE IF NOT EXISTS game_reward_funding (
  allocation_hash TEXT PRIMARY KEY NOT NULL REFERENCES game_reward_intents(allocation_hash),
  budget_id TEXT NOT NULL UNIQUE, evidence_digest TEXT NOT NULL UNIQUE,
  proof_json TEXT NOT NULL CHECK(json_valid(proof_json)),
  actor_id TEXT NOT NULL REFERENCES users(id), recorded_ms INTEGER NOT NULL
);
