variable "name" {
  type    = string
  default = "migrator"
}

variable "docker_image" {
  type = string
}

variable "namespace" {
  type    = string
  default = "scuffle-migrations"
}

variable "tls_secret_name" {
  type = string
}

variable "postgres_host" {
  type = string
}

variable "postgres_username" {
  type = string
}

variable "postgres_database" {
  type = string
}

variable "migrations_path" {
  type = string
}
