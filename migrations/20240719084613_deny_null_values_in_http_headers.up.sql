-- Add up migration script here
UPDATE "http-recon" SET headers = '{}'::jsonb WHERE jsonb_typeof(headers) <> 'object';
UPDATE "https-recon" SET headers = '{}'::jsonb WHERE jsonb_typeof(headers) <> 'object';
ALTER TABLE "http-recon" ALTER COLUMN headers SET DEFAULT '{}'::jsonb;
ALTER TABLE "https-recon" ALTER COLUMN headers SET DEFAULT '{}'::jsonb;
ALTER TABLE "http-recon" ALTER COLUMN headers SET NOT NULL;
ALTER TABLE "https-recon" ALTER COLUMN headers SET NOT NULL;
