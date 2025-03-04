// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use std::sync::Arc;

use async_trait::async_trait;
use http::StatusCode;

use super::core::AzdlsCore;
use super::error::parse_error;
use crate::raw::oio::WriteBuf;
use crate::raw::*;
use crate::*;

pub struct AzdlsWriter {
    core: Arc<AzdlsCore>,

    op: OpWrite,
    path: String,
}

impl AzdlsWriter {
    pub fn new(core: Arc<AzdlsCore>, op: OpWrite, path: String) -> Self {
        AzdlsWriter { core, op, path }
    }
}

#[async_trait]
impl oio::OneShotWrite for AzdlsWriter {
    async fn write_once(&self, bs: &dyn WriteBuf) -> Result<()> {
        let mut req =
            self.core
                .azdls_create_request(&self.path, "file", &self.op, AsyncBody::Empty)?;

        self.core.sign(&mut req).await?;

        let resp = self.core.send(req).await?;

        let status = resp.status();
        match status {
            StatusCode::CREATED | StatusCode::OK => {
                resp.into_body().consume().await?;
            }
            _ => {
                return Err(parse_error(resp)
                    .await?
                    .with_operation("Backend::azdls_create_request"));
            }
        }

        let bs = oio::ChunkedBytes::from_vec(bs.vectored_bytes(bs.remaining()));
        let mut req = self.core.azdls_update_request(
            &self.path,
            Some(bs.len()),
            AsyncBody::ChunkedBytes(bs),
        )?;

        self.core.sign(&mut req).await?;

        let resp = self.core.send(req).await?;

        let status = resp.status();
        match status {
            StatusCode::OK | StatusCode::ACCEPTED => {
                resp.into_body().consume().await?;
                Ok(())
            }
            _ => Err(parse_error(resp)
                .await?
                .with_operation("Backend::azdls_update_request")),
        }
    }
}
