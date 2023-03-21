variable "docker_image" {
  type = string
}

variable "namespace" {
  type    = string
  default = "scuffle-website"
}

variable "api_host" {
  type = string
}
