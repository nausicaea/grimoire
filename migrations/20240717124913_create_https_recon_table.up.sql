-- Add up migration script here
CREATE TABLE "https-recon" (id SERIAL, fqdn varchar(256) REFERENCES "cert-recon"("cert-name"), url text, "response-status" smallint, headers jsonb);