group "default" {
  targets = ["seitai"]
}

variable "VERSION" {
  default = "latest"
}

target "seitai" {
  target = "seitai"
  tags = [
    "ghcr.io/hexium310/seitai:latest",
    "ghcr.io/hexium310/seitai:${VERSION}"
  ]
}
