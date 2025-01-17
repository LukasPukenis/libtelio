/* ----------------------------------------------------------------------------
 * This file was automatically generated by SWIG (http://www.swig.org).
 * Version 4.0.2
 *
 * Do not make changes to this file unless you know what you are doing--modify
 * the SWIG interface file instead.
 * ----------------------------------------------------------------------------- */

package com.nordsec.telio;

public class libtelioJNI {
  public final static native int LOG_CRITICAL_get();
  public final static native int LOG_ERROR_get();
  public final static native int LOG_WARNING_get();
  public final static native int LOG_INFO_get();
  public final static native int LOG_DEBUG_get();
  public final static native int LOG_TRACE_get();
  public final static native int RES_OK_get();
  public final static native int RES_ERROR_get();
  public final static native int RES_INVALID_KEY_get();
  public final static native int RES_BAD_CONFIG_get();
  public final static native int RES_LOCK_ERROR_get();
  public final static native int RES_INVALID_STRING_get();
  public final static native int RES_ALREADY_STARTED_get();
  public final static native long new_Telio(String jarg1, ITelioEventCb jarg2, int jarg3, ITelioLoggerCb jarg4, ITelioProtectCb jarg5);
  public final static native int Telio_getDefaultAdapter();
  public final static native void delete_Telio(long jarg1);
  public final static native int Telio_start(long jarg1, Telio jarg1_, String jarg2, int jarg3);
  public final static native int Telio_startNamed(long jarg1, Telio jarg1_, String jarg2, int jarg3, String jarg4);
  public final static native int Telio_startWithTun(long jarg1, Telio jarg1_, String jarg2, int jarg3, int jarg4);
  public final static native int Telio_enableMagicDns(long jarg1, Telio jarg1_, String jarg2);
  public final static native int Telio_disableMagicDns(long jarg1, Telio jarg1_);
  public final static native int Telio_stop(long jarg1, Telio jarg1_);
  public final static native java.math.BigInteger Telio_getAdapterLuid(long jarg1, Telio jarg1_);
  public final static native int Telio_setPrivateKey(long jarg1, Telio jarg1_, String jarg2);
  public final static native String Telio_getPrivateKey(long jarg1, Telio jarg1_);
  public final static native int Telio_notifyNetworkChange(long jarg1, Telio jarg1_, String jarg2);
  public final static native int Telio_connectToExitNode(long jarg1, Telio jarg1_, String jarg2, String jarg3, String jarg4);
  public final static native int Telio_disconnectFromExitNode(long jarg1, Telio jarg1_, String jarg2);
  public final static native int Telio_disconnectFromExitNodes(long jarg1, Telio jarg1_);
  public final static native int Telio_setMeshnet(long jarg1, Telio jarg1_, String jarg2);
  public final static native int Telio_setMeshnetOff(long jarg1, Telio jarg1_);
  public final static native String Telio_generateSecretKey(long jarg1, Telio jarg1_);
  public final static native String Telio_generatePublicKey(long jarg1, Telio jarg1_, String jarg2);
  public final static native String Telio_getStatusMap(long jarg1, Telio jarg1_);
  public final static native String Telio_getLastError(long jarg1, Telio jarg1_);
  public final static native String Telio_getVersionTag();
  public final static native String Telio_getCommitSha();
}
