//! The views module contains the components for all Layouts and Routes for our app. Each layout and route in our [`Route`]
//! enum will render one of these components.
//!
//!
//! The [`Home`] and [`Blog`] components will be rendered when the current route is [`Route::Home`] or [`Route::Blog`] respectively.
//!
//!
//! The [`Navbar`] component will be rendered on all pages of our app since every page is under the layout. The layout defines
//! a common wrapper around all child routes.

mod home;
pub use home::Home;

mod navbar;
pub use navbar::Navbar;

mod for_nerds;
pub use for_nerds::ForNerds;

mod voguguru;
pub use voguguru::VoguGuru;

mod gallery;
pub use gallery::Gallery;

pub mod gallery_client;
mod makingof;
pub use makingof::MakingOf;

mod how_it_works;
pub use how_it_works::HowItWorks;

mod newsletter;
pub use newsletter::Newsletter;

mod unsubscribe;
pub use unsubscribe::Unsubscribe;

mod admin;
pub use admin::Admin;
