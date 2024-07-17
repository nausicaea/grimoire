-- Add up migration script here
CREATE TABLE "cert-recon" (id SERIAL, domain varchar(256), "cert-name" varchar(256) PRIMARY KEY);
