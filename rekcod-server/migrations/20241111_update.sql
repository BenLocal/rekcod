CREATE UNIQUE INDEX IF NOT EXISTS "kvs_module_third_key_idx_unique" ON "kvs" ("third_key", "sub_key", "key", "module");