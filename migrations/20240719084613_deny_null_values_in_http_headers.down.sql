-- Add down migration script here
ALTER TABLE "http-recon" ALTER COLUMN headers DROP NOT NULL;
ALTER TABLE "https-recon" ALTER COLUMN headers DROP NOT NULL;
ALTER TABLE "http-recon" ALTER COLUMN headers DROP DEFAULT;
ALTER TABLE "https-recon" ALTER COLUMN headers DROP DEFAULT;
