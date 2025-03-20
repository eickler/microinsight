# Changelog

## [0.7.14](https://github.com/eickler/microinsight/compare/v0.7.13...v0.7.14) (2025-03-20)


### Bug Fixes

* Detect stuck threads ([0a0c07e](https://github.com/eickler/microinsight/commit/0a0c07ecb3364f82b7631bafd4b981087fe9f642))

## [0.7.13](https://github.com/eickler/microinsight/compare/v0.7.12...v0.7.13) (2025-03-20)


### Bug Fixes

* Suspect: pymysqlpool locks up on retries ([2f76e26](https://github.com/eickler/microinsight/commit/2f76e26d901ef83f2c51888bfcffcb8586ea0a82))

## [0.7.12](https://github.com/eickler/microinsight/compare/v0.7.11...v0.7.12) (2025-03-20)


### Bug Fixes

* Prevent pool exhaustion, discard too old data ([c1d2a91](https://github.com/eickler/microinsight/commit/c1d2a91b5c20e6d6d7127da0da534349e5c1e720))

## [0.7.11](https://github.com/eickler/microinsight/compare/v0.7.10...v0.7.11) (2025-03-20)


### Bug Fixes

* Suspect: Running out of connections ([6f63155](https://github.com/eickler/microinsight/commit/6f63155ca32b033afa68b1c4295e20910b856df9))

## [0.7.10](https://github.com/eickler/microinsight/compare/v0.7.9...v0.7.10) (2025-03-19)


### Bug Fixes

* Debug logging ([78ba087](https://github.com/eickler/microinsight/commit/78ba087ebe7cfc2c3844671c50ddca366404c8b5))

## [0.7.9](https://github.com/eickler/microinsight/compare/v0.7.8...v0.7.9) (2025-03-19)


### Bug Fixes

* More logging ([a9541ab](https://github.com/eickler/microinsight/commit/a9541ab60ee048e305d655b24f9e0383891ca8bd))

## [0.7.8](https://github.com/eickler/microinsight/compare/v0.7.7...v0.7.8) (2025-03-19)


### Bug Fixes

* Fixed connection setup ([ac7314a](https://github.com/eickler/microinsight/commit/ac7314a30d2e3bd1a5178dd28a4f9c544bb8d535))

## [0.7.7](https://github.com/eickler/microinsight/compare/v0.7.6...v0.7.7) (2025-03-19)


### Bug Fixes

* Fixed locking ([a9ea5e4](https://github.com/eickler/microinsight/commit/a9ea5e4b7fb665dec79ccaccf6630a865c9ac03d))

## [0.7.6](https://github.com/eickler/microinsight/compare/v0.7.5...v0.7.6) (2025-03-19)


### Bug Fixes

* Try requesting CPU ([769f764](https://github.com/eickler/microinsight/commit/769f7640e40b5d5782218b22c815476da069274a))

## [0.7.5](https://github.com/eickler/microinsight/compare/v0.7.4...v0.7.5) (2025-03-19)


### Bug Fixes

* Reducing logging output size ([a358ffb](https://github.com/eickler/microinsight/commit/a358ffbfa08dc0bd15d36c726b0a49894ededaa7))

## [0.7.4](https://github.com/eickler/microinsight/compare/v0.7.3...v0.7.4) (2025-03-19)


### Bug Fixes

* Another logging attempt ([19e7142](https://github.com/eickler/microinsight/commit/19e71427bd4b419de06423c9b51e74ec5c9c1a38))

## [0.7.3](https://github.com/eickler/microinsight/compare/v0.7.2...v0.7.3) (2025-03-19)


### Bug Fixes

* Added temporary debug logging ([8aa2845](https://github.com/eickler/microinsight/commit/8aa28458d3559af77a5f7e45a7db4f4faf19095b))

## [0.7.2](https://github.com/eickler/microinsight/compare/v0.7.1...v0.7.2) (2025-02-24)


### Bug Fixes

* Fixed THREADS parameter, fixed empty timeseries ([b62cc61](https://github.com/eickler/microinsight/commit/b62cc612ec3426f58fa0e026152be7ed9ab4393b))

## [0.7.1](https://github.com/eickler/microinsight/compare/v0.7.0...v0.7.1) (2025-02-24)


### Bug Fixes

* Convert THREADS to number ([ab28b37](https://github.com/eickler/microinsight/commit/ab28b37a189b3d63e0bc3f672e55a17e05647b7d))

## [0.7.0](https://github.com/eickler/microinsight/compare/v0.6.1...v0.7.0) (2025-02-17)


### Features

* Improved logging ([7c75a9a](https://github.com/eickler/microinsight/commit/7c75a9a86ed8f913723a70397b5402cce06a92ad))
* More default capacity, logging improvements ([6d7a571](https://github.com/eickler/microinsight/commit/6d7a571f5553912f271d9cdeaf8dae34a018c976))
* Write owners once every OWNER_FLUSH_INTERVAL secs ([e96c5df](https://github.com/eickler/microinsight/commit/e96c5df8611615b03dde780f52ac213d9118f5e9))

## [0.6.1](https://github.com/eickler/microinsight/compare/v0.6.0...v0.6.1) (2025-02-12)


### Bug Fixes

* Filter NaN values in Prometheus time series ([7dd26ca](https://github.com/eickler/microinsight/commit/7dd26caa87a3e0316fd393c23dd3d79c641630d3))

## [0.6.0](https://github.com/eickler/microinsight/compare/v0.5.1...v0.6.0) (2025-02-10)


### Features

* Added configuration parameters, improved docs ([9a066a3](https://github.com/eickler/microinsight/commit/9a066a35a39ba4f1466f2cfe17a6e53365858f23))

## [0.5.1](https://github.com/eickler/microinsight/compare/v0.5.0...v0.5.1) (2025-02-09)


### Bug Fixes

* Container image reference fixed ([adaf82f](https://github.com/eickler/microinsight/commit/adaf82f19b78f8ac01d905ed95da18293a292e7d))

## [0.5.0](https://github.com/eickler/microinsight/compare/v0.4.3...v0.5.0) (2025-02-08)


### Features

* Blacklisting of system pods ([cb5b99f](https://github.com/eickler/microinsight/commit/cb5b99f7b3e8933fa7769c1ea6915eb005cd9a43))
* Configurable web server thread pool ([594171d](https://github.com/eickler/microinsight/commit/594171d15d104f9e159e9ccb485c28ad01010377))
* First skech of remote writer ([8767133](https://github.com/eickler/microinsight/commit/87671331d9d7d71dc76c256a694828b7b37f54be))
* Interval configuration added, README written ([bb3454d](https://github.com/eickler/microinsight/commit/bb3454d6fe124367a6579be241a8744cde8398ca))
* Limit batch size ([596e9db](https://github.com/eickler/microinsight/commit/596e9db33e2d77ed8e61da222f95ea8c19b283b3))
* Separate between metrics and owner ([79f9359](https://github.com/eickler/microinsight/commit/79f935948f7d408b7357053cf57675761b98e54e))
* Use version tag instead of latest for images ([0992046](https://github.com/eickler/microinsight/commit/0992046d5f44aa2c04dcb2e9301f5f6855083e28))


### Bug Fixes

* *bleep* this, using rust type ([d6ec2f2](https://github.com/eickler/microinsight/commit/d6ec2f22d25bfcdfc7dc215577511f7fdfc2a4b2))
* Add empty manifest ([da26983](https://github.com/eickler/microinsight/commit/da26983fa284d3a91ca11950b1ace5fb7abe72ec))
* Added build image to compile cryptography ([9e103be](https://github.com/eickler/microinsight/commit/9e103be69c1a9ea4e838fbeb69bc4e534896b462))
* Added empty brackets ([b4e8642](https://github.com/eickler/microinsight/commit/b4e86421077c8ed071e125194a874688495fd7c4))
* Added version.txt ([880d9f4](https://github.com/eickler/microinsight/commit/880d9f48892c83b7dfa8db9b1623e716dcb13acc))
* Another try ([9da278e](https://github.com/eickler/microinsight/commit/9da278e5e1bc86fa1f8fcb91a1cc8be15cb45a9c))
* Another try ([698e524](https://github.com/eickler/microinsight/commit/698e52478975d0a438bf9a8f1b37f13bcdbce774))
* Bla ([c188213](https://github.com/eickler/microinsight/commit/c18821369324a801c7c4fe049ef4f150e524da87))
* Bla ([0a996e4](https://github.com/eickler/microinsight/commit/0a996e47b1f48a6e9c5f6f7929826364353b9892))
* Bla2 ([9379ae8](https://github.com/eickler/microinsight/commit/9379ae8a7d5cb9bcfa9241ed00e33025426bef13))
* Cast interval to correct type ([edac1c0](https://github.com/eickler/microinsight/commit/edac1c04a6ecac0e47db237090c3d90afc7d7a73))
* Default interval reduced ([fa82690](https://github.com/eickler/microinsight/commit/fa826908293942724dab10c455f024926d829142))
* Fixed the release-please configuration ([56c8ae9](https://github.com/eickler/microinsight/commit/56c8ae9aa4b58fa6ec7224b2890bcc977fb6548a))
* Fixing manifest again ([a6f5c2c](https://github.com/eickler/microinsight/commit/a6f5c2c8d93e5de4c9059a945981bebeede1f049))
* Fixing manifest again ([7532936](https://github.com/eickler/microinsight/commit/75329368cd8c8144652fd034f50343b4a938b6d0))
* Fixing manifest again ([8ace8c9](https://github.com/eickler/microinsight/commit/8ace8c956f37c6639752dfcc7014caefe6e036de))
* Fixing manifest again ([e29c7aa](https://github.com/eickler/microinsight/commit/e29c7aa717daa58222f8413a1ecae5b053d9bd83))
* Fixing the actions flow ([411ef0d](https://github.com/eickler/microinsight/commit/411ef0d3981264050fd70c418e77bfa0ace4b87e))
* Forgot services, small fixes ([5b36799](https://github.com/eickler/microinsight/commit/5b367992a3dbf097de8dd8f2d0379783f6cb9e8c))
* Forgot to delete the Chart manipulation action ([877d4d4](https://github.com/eickler/microinsight/commit/877d4d436591f3dbb762054fb407f7eaa7cb7cb1))
* Helm type seems to mess up version prefix ([0254dcc](https://github.com/eickler/microinsight/commit/0254dcc8952023dea384405d4e9cc16278858e15))
* Ignore write_requests that have no pod set. ([7d16374](https://github.com/eickler/microinsight/commit/7d16374431646ca4dfd45de53a5597f7b84b1f77))
* Log level setting fixed ([b6f981b](https://github.com/eickler/microinsight/commit/b6f981bc9ca66235bb817fff30e22b5de4419d23))
* Reduction of column length due to MySQL limit ([e7ea154](https://github.com/eickler/microinsight/commit/e7ea154647bf5187db51e7bb8279f0fc80ed4915))
* Test cases and various fixes ([243efce](https://github.com/eickler/microinsight/commit/243efce81f697f0e078d46ec79178080e45c4099))
* Test fix ([8cdaed5](https://github.com/eickler/microinsight/commit/8cdaed57d05cadd24f6aa34acd36678d32546e5f))
* Try explicit manifest creation ([ce54e27](https://github.com/eickler/microinsight/commit/ce54e27088db043f70a5c9d34a25aed249d7243a))
* Try re-releasing 0.4.2 ([731645b](https://github.com/eickler/microinsight/commit/731645b49fba5d382e7091b7fd93aff258cde3ea))
* Try root path in config file ([c26214c](https://github.com/eickler/microinsight/commit/c26214ce63a1605e1248390e879aeb15bc892bf7))
* Try skipping 0.4.2 ([b280304](https://github.com/eickler/microinsight/commit/b280304a91c9aab7e099f0fe9ac0ab4fc012536a))
* Try with versioning prefix ([db37b8e](https://github.com/eickler/microinsight/commit/db37b8eaedc9b9a34fcdde8c1a83a9e70ac810bd))
* Various bug fixes and unit test ([f2989c4](https://github.com/eickler/microinsight/commit/f2989c49cfbdec6e5c71d2885e5fb128fd0c9a27))
* Various timestamp handling fixes ([a053ac8](https://github.com/eickler/microinsight/commit/a053ac8a67dde9ddabc6b896b6ec4fbe72b059f3))
* Workaround for the v limitation in the versions ([2a5ac13](https://github.com/eickler/microinsight/commit/2a5ac1372dc302a2c5a64e66e9c75fd9800b1f0b))

## [0.4.2](https://github.com/eickler/microinsight/compare/v0.4.1...v0.4.2) (2025-02-03)


### Bug Fixes

* Bla ([0a996e4](https://github.com/eickler/microinsight/commit/0a996e47b1f48a6e9c5f6f7929826364353b9892))

## [0.4.1](https://github.com/eickler/microinsight/compare/v0.4.0...v0.4.1) (2025-02-03)


### Bug Fixes

* Helm type seems to mess up version prefix ([0254dcc](https://github.com/eickler/microinsight/commit/0254dcc8952023dea384405d4e9cc16278858e15))

## [0.4.0](https://github.com/eickler/microinsight/compare/v0.3.0...v0.4.0) (2025-02-03)


### Features

* Use version tag instead of latest for images ([0992046](https://github.com/eickler/microinsight/commit/0992046d5f44aa2c04dcb2e9301f5f6855083e28))


### Bug Fixes

* *bleep* this, using rust type ([d6ec2f2](https://github.com/eickler/microinsight/commit/d6ec2f22d25bfcdfc7dc215577511f7fdfc2a4b2))
* Add empty manifest ([da26983](https://github.com/eickler/microinsight/commit/da26983fa284d3a91ca11950b1ace5fb7abe72ec))
* Added empty brackets ([b4e8642](https://github.com/eickler/microinsight/commit/b4e86421077c8ed071e125194a874688495fd7c4))
* Added version.txt ([880d9f4](https://github.com/eickler/microinsight/commit/880d9f48892c83b7dfa8db9b1623e716dcb13acc))
* Another try ([9da278e](https://github.com/eickler/microinsight/commit/9da278e5e1bc86fa1f8fcb91a1cc8be15cb45a9c))
* Another try ([698e524](https://github.com/eickler/microinsight/commit/698e52478975d0a438bf9a8f1b37f13bcdbce774))
* Fixed the release-please configuration ([56c8ae9](https://github.com/eickler/microinsight/commit/56c8ae9aa4b58fa6ec7224b2890bcc977fb6548a))
* Fixing manifest again ([a6f5c2c](https://github.com/eickler/microinsight/commit/a6f5c2c8d93e5de4c9059a945981bebeede1f049))
* Fixing manifest again ([7532936](https://github.com/eickler/microinsight/commit/75329368cd8c8144652fd034f50343b4a938b6d0))
* Fixing manifest again ([8ace8c9](https://github.com/eickler/microinsight/commit/8ace8c956f37c6639752dfcc7014caefe6e036de))
* Fixing manifest again ([e29c7aa](https://github.com/eickler/microinsight/commit/e29c7aa717daa58222f8413a1ecae5b053d9bd83))
* Ignore write_requests that have no pod set. ([7d16374](https://github.com/eickler/microinsight/commit/7d16374431646ca4dfd45de53a5597f7b84b1f77))
* Log level setting fixed ([b6f981b](https://github.com/eickler/microinsight/commit/b6f981bc9ca66235bb817fff30e22b5de4419d23))
* Test fix ([8cdaed5](https://github.com/eickler/microinsight/commit/8cdaed57d05cadd24f6aa34acd36678d32546e5f))
* Try explicit manifest creation ([ce54e27](https://github.com/eickler/microinsight/commit/ce54e27088db043f70a5c9d34a25aed249d7243a))
* Try root path in config file ([c26214c](https://github.com/eickler/microinsight/commit/c26214ce63a1605e1248390e879aeb15bc892bf7))

## [0.3.0](https://github.com/eickler/microinsight/compare/v0.2.2...v0.3.0) (2025-01-22)


### Features

* Blacklisting of system pods ([cb5b99f](https://github.com/eickler/microinsight/commit/cb5b99f7b3e8933fa7769c1ea6915eb005cd9a43))

## [0.2.2](https://github.com/eickler/microinsight/compare/v0.2.1...v0.2.2) (2025-01-22)


### Bug Fixes

* Default interval reduced ([fa82690](https://github.com/eickler/microinsight/commit/fa826908293942724dab10c455f024926d829142))

## [0.2.1](https://github.com/eickler/microinsight/compare/v0.2.0...v0.2.1) (2025-01-15)


### Bug Fixes

* Cast interval to correct type ([edac1c0](https://github.com/eickler/microinsight/commit/edac1c04a6ecac0e47db237090c3d90afc7d7a73))

## [0.2.0](https://github.com/eickler/microinsight/compare/v0.1.2...v0.2.0) (2024-07-10)


### Features

* Separate between metrics and owner ([79f9359](https://github.com/eickler/microinsight/commit/79f935948f7d408b7357053cf57675761b98e54e))


### Bug Fixes

* Reduction of column length due to MySQL limit ([e7ea154](https://github.com/eickler/microinsight/commit/e7ea154647bf5187db51e7bb8279f0fc80ed4915))

## [0.1.2](https://github.com/eickler/microinsight/compare/v0.1.1...v0.1.2) (2024-07-08)


### Bug Fixes

* Forgot to delete the Chart manipulation action ([877d4d4](https://github.com/eickler/microinsight/commit/877d4d436591f3dbb762054fb407f7eaa7cb7cb1))

## [0.1.1](https://github.com/eickler/microinsight/compare/v0.1.0...v0.1.1) (2024-07-08)


### Bug Fixes

* Fixing the actions flow ([411ef0d](https://github.com/eickler/microinsight/commit/411ef0d3981264050fd70c418e77bfa0ace4b87e))

## 0.1.0 (2024-07-08)


### Features

* First skech of remote writer ([8767133](https://github.com/eickler/microinsight/commit/87671331d9d7d71dc76c256a694828b7b37f54be))
* Interval configuration added, README written ([bb3454d](https://github.com/eickler/microinsight/commit/bb3454d6fe124367a6579be241a8744cde8398ca))


### Bug Fixes

* Added build image to compile cryptography ([9e103be](https://github.com/eickler/microinsight/commit/9e103be69c1a9ea4e838fbeb69bc4e534896b462))
* Forgot services, small fixes ([5b36799](https://github.com/eickler/microinsight/commit/5b367992a3dbf097de8dd8f2d0379783f6cb9e8c))
