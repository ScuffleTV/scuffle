variable "api_docker_image" {
  type    = string
  default = "ghcr.io/scuffletv/api:latest"
}

variable "edge_docker_image" {
  type    = string
  default = "ghcr.io/scuffletv/edge:latest"
}

variable "website_docker_image" {
  type    = string
  default = "ghcr.io/scuffletv/website:latest"
}

variable "transcoder_docker_image" {
  type    = string
  default = "ghcr.io/scuffletv/transcoder:latest"
}

variable "ingest_docker_image" {
  type    = string
  default = "ghcr.io/scuffletv/ingest:latest"
}

data "terraform_remote_state" "scuffle_infra" {
  backend = "remote"

  config = {
    organization = "scuffle"
    workspaces = {
      name = "scuffle-infra-${trimprefix(terraform.workspace, "scuffle-app-")}"
    }
  }
}

variable "turnstile_secret_key" {
  type      = string
  sensitive = true
}
