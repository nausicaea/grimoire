-- Add up migration script here
ALTER TABLE "dns-recon" ADD COLUMN domain VARCHAR(256);
UPDATE "dns-recon" SET domain = regexp_replace(fqdn, '^(?:[a-zA-Z0-9-]+\.)*((?:[a-zA-Z0-9-]+\.)(?:[a-zA-Z0-9-]+))$', '\1');
