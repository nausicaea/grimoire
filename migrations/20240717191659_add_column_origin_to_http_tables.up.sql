-- Add up migration script here
ALTER TABLE "http-recon" ADD COLUMN domain VARCHAR(256);
ALTER TABLE "https-recon" ADD COLUMN domain VARCHAR(256);
