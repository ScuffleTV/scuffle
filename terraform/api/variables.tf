variable "docker_image" {
  type = string
}

variable "namespace" {
  type    = string
  default = "scuffle-api"
}

variable "scuffle_infra" {
  type = any
}

variable "turnstile_secret_key" {
  type      = string
  sensitive = true
}
