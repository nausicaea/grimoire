-- Add down migration script here
ALTER TABLE "http-recon" DROP COLUMN domain;
ALTER TABLE "https-recon" DROP COLUMN domain;
