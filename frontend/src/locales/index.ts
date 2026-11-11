import zhCommon from './common.zh'
import enCommon from './common.en'
import zhLayout from './layout.zh'
import enLayout from './layout.en'
import zhAuth from './auth.zh'
import enAuth from './auth.en'
import zhInvite from './invite.zh'
import enInvite from './invite.en'
import zhHome from './home.zh'
import enHome from './home.en'
import zhRides from './rides.zh'
import enRides from './rides.en'
import zhRideCard from './rideCard.zh'
import enRideCard from './rideCard.en'
import zhRideDetail from './rideDetail.zh'
import enRideDetail from './rideDetail.en'
import zhRideNew from './rideNew.zh'
import enRideNew from './rideNew.en'
import zhDrive from './drive.zh'
import enDrive from './drive.en'
import zhEvents from './events.zh'
import enEvents from './events.en'
import zhEventDetail from './eventDetail.zh'
import enEventDetail from './eventDetail.en'
import zhEventForm from './eventForm.zh'
import enEventForm from './eventForm.en'
import zhSettlements from './settlements.zh'
import enSettlements from './settlements.en'
import zhCosts from './costs.zh'
import enCosts from './costs.en'
import zhReimbursements from './reimbursements.zh'
import enReimbursements from './reimbursements.en'
import zhHelp from './help.zh'
import enHelp from './help.en'
import zhNotifications from './notifications.zh'
import enNotifications from './notifications.en'
import zhProfile from './profile.zh'
import enProfile from './profile.en'
import zhReport from './report.zh'
import enReport from './report.en'
import zhAdmin from './admin.zh'
import enAdmin from './admin.en'
import zhFormat from './format.zh'
import enFormat from './format.en'
import zhErrors from './errors.zh'
import enErrors from './errors.en'
import zhTerms from './terms.zh'
import enTerms from './terms.en'
import zhNotFound from './notFound.zh'
import enNotFound from './notFound.en'
import zhChrome from './chrome.zh'
import enChrome from './chrome.en'

export const namespaces = [
  'common',
  'layout',
  'auth',
  'invite',
  'home',
  'rides',
  'rideCard',
  'rideDetail',
  'rideNew',
  'drive',
  'events',
  'eventDetail',
  'eventForm',
  'settlements',
  'costs',
  'reimbursements',
  'help',
  'notifications',
  'profile',
  'report',
  'admin',
  'format',
  'errors',
  'terms',
  'notFound',
  'chrome',
] as const

export type Namespace = (typeof namespaces)[number]

export const resources = {
  en: {
    common: enCommon,
    layout: enLayout,
    auth: enAuth,
    invite: enInvite,
    home: enHome,
    rides: enRides,
    rideCard: enRideCard,
    rideDetail: enRideDetail,
    rideNew: enRideNew,
    drive: enDrive,
    events: enEvents,
    eventDetail: enEventDetail,
    eventForm: enEventForm,
    settlements: enSettlements,
    costs: enCosts,
    reimbursements: enReimbursements,
    help: enHelp,
    notifications: enNotifications,
    profile: enProfile,
    report: enReport,
    admin: enAdmin,
    format: enFormat,
    errors: enErrors,
    terms: enTerms,
    notFound: enNotFound,
    chrome: enChrome,
  },
  zh: {
    common: zhCommon,
    layout: zhLayout,
    auth: zhAuth,
    invite: zhInvite,
    home: zhHome,
    rides: zhRides,
    rideCard: zhRideCard,
    rideDetail: zhRideDetail,
    rideNew: zhRideNew,
    drive: zhDrive,
    events: zhEvents,
    eventDetail: zhEventDetail,
    eventForm: zhEventForm,
    settlements: zhSettlements,
    costs: zhCosts,
    reimbursements: zhReimbursements,
    help: zhHelp,
    notifications: zhNotifications,
    profile: zhProfile,
    report: zhReport,
    admin: zhAdmin,
    format: zhFormat,
    errors: zhErrors,
    terms: zhTerms,
    notFound: zhNotFound,
    chrome: zhChrome,
  },
} as const
