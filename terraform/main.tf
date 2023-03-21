terraform {
  backend "remote" {
    hostname     = "app.terraform.io"
    organization = "scuffle"

    workspaces {
      prefix = "scuffle-app-"
    }
  }
}

module "api" {
  source               = "./api"
  docker_image         = var.api_docker_image
  scuffle_infra        = data.terraform_remote_state.scuffle_infra
  turnstile_secret_key = var.turnstile_secret_key
}

module "edge" {
  source       = "./edge"
  docker_image = var.edge_docker_image
  depends_on = [
    module.api # Wait for the API to be ready
  ]
}

module "ingest" {
  source       = "./ingest"
  docker_image = var.ingest_docker_image
  depends_on = [
    module.api # Wait for the API to be ready
  ]
}

module "transcoder" {
  source       = "./transcoder"
  docker_image = var.transcoder_docker_image
  depends_on = [
    module.api # Wait for the API to be ready
  ]
}

module "website" {
  source       = "./website"
  docker_image = var.website_docker_image
  api_host     = module.api.hostname
  depends_on = [
    module.api # Wait for the API to be ready
  ]
}
