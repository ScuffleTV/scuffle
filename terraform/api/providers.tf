terraform {
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "2.18.1"
    }
    random = {
      source  = "hashicorp/random"
      version = "3.4.3"
    }
  }
}
