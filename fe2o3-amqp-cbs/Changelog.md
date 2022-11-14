# Change Log

## 0.0.9

- Updated dependencies `fe2o3-amqp-management` to "0.0.4" and updated `PutTokenRequest` and
  `PutTokenResponse` to use `Request` and `Response` traits

## 0.0.8

- Experimental use of GAT as opposed to `Pin<Box<dyn Future>>`

## 0.0.7

- Changed trait signature

## 0.0.6

- Added lifetime marker to `AsyncCbsTokenProvider`

## 0.0.5

- Moved `name` outside `CbsToken`

## 0.0.4

- Fixed error in `AsyncCbsTokenProvider`

## 0.0.3

- Added `CbsToken` struct, and `CbsTokenProvider`, `AsyncCbsTokenProvider` traits.
- Changed `Client::put_token` to take a `CbsToken` instead.
