










#[cfg(test)]
mod tests {
    use super::*;

    use apca::data::v2::stream::{Bar, Quote, RealtimeData, Trade, IEX};
    use http::StatusCode;

    use apca;
    use apca::ApiError;

    /* Endpoint! {
      GetNotFound(()),
      Ok => (), [],
      Err => GetNotFoundError, []

      fn path(_input: &Self::Input) -> Str {
        "/v2/foobarbaz".into()
      }
    } */
    #[test]
    fn eee() {}
    /// Check that we can retrieve the `ApiInfo` object used by a client.
    #[tokio::test]
    async fn client_api_info() {

        /* let api_info = ApiInfo::from_env().unwrap();
        let client = Client::builder().build(api_info.clone());
        assert_eq!(&api_info, client.api_info()); */
    }

    /// Check that formatting a [`DebugRequest`] masks secret values.
    #[test]
    fn request_debugging() {
        /* let api_info = ApiInfo::from_env().unwrap();
        let client = Client::builder().build(api_info);

        let request = client.request::<GetNotFound>(&()).unwrap();
        let value = debug_request(&request);
        let string = format!("{value:?}");
        assert!(string.contains("<masked>"), "{string}"); */
    }

    /*  /// Check basic workings of the HTTP status evaluation logic.
    #[test(tokio::test)]
    async fn unexpected_status_code_return() {
        let api_info = ApiInfo::from_env().unwrap();
        let client = Client::builder().max_idle_per_host(0).build(api_info);
        let result = client.issue::<GetNotFound>(&()).await;
        let err = result.unwrap_err();

        match err {
            RequestError::Endpoint(GetNotFoundError::UnexpectedStatus(status, message)) => {
                let expected = ApiError {
                    message: "endpoint not found".to_string(),
                };
                assert_eq!(message, Ok(expected));
                assert_eq!(status, StatusCode::NOT_FOUND);
            }
            _ => panic!("Received unexpected error: {err:?}"),
        };
    } */
}
